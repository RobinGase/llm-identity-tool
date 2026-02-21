use crate::persona::Persona;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const EPHEMERAL_TTL_MS: i64 = 7 * 24 * 60 * 60 * 1000;

const FIRST_NAMES: [&str; 16] = [
    "Aiden", "Noah", "Mason", "Elias", "Luca", "Ethan", "Mila", "Ava", "Sofia", "Nora", "Aria",
    "Lena", "Kai", "Rin", "Nova", "Quinn",
];

const LAST_NAMES: [&str; 16] = [
    "Harper", "Bennett", "Walker", "Hayes", "Reed", "Parker", "Morgan", "Rivera", "Brooks",
    "Foster", "Campbell", "Shaw", "Sullivan", "Hayden", "Lane", "Rowe",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaRecord {
    pub model_id: String,
    pub persona: Persona,
    pub first_name: String,
    pub last_name: String,
    pub birth_unix_ms: i64,
    pub persistent: bool,
    pub expires_at_unix_ms: Option<i64>,
    pub last_seen_unix_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectResult {
    pub persona: Persona,
    pub model_id: String,
    pub first_name: String,
    pub last_name: String,
    pub birth_unix_ms: i64,
    pub persistent: bool,
    pub expires_at_unix_ms: Option<i64>,
    pub created: bool,
}

impl ConnectResult {
    fn from_record(record: PersonaRecord, created: bool) -> Self {
        Self {
            persona: record.persona,
            model_id: record.model_id,
            first_name: record.first_name,
            last_name: record.last_name,
            birth_unix_ms: record.birth_unix_ms,
            persistent: record.persistent,
            expires_at_unix_ms: record.expires_at_unix_ms,
            created,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct PersonaRegistryFile {
    entries: Vec<PersonaRecord>,
}

pub struct PersonaRegistry {
    path: PathBuf,
    store: PersonaRegistryFile,
}

impl PersonaRegistry {
    pub fn load_default() -> Result<Self> {
        let mut path = dirs::home_dir().context("Could not find home directory")?;
        path.push(".kaispeech_personas.json");
        Self::load_with_path(path)
    }

    pub fn load_with_path(path: PathBuf) -> Result<Self> {
        let store = if path.exists() {
            let content = fs::read_to_string(&path)?;
            let content = content.trim_start_matches('\u{feff}');
            if content.trim().is_empty() {
                PersonaRegistryFile::default()
            } else {
                serde_json::from_str(content).with_context(|| {
                    format!("Failed to parse persona registry at {}", path.display())
                })?
            }
        } else {
            PersonaRegistryFile::default()
        };

        let mut registry = Self { path, store };
        let _ = registry.prune_expired()?;
        Ok(registry)
    }

    pub fn count(&self) -> usize {
        self.store.entries.len()
    }

    pub fn connect_or_create(&mut self, model_id: &str, persistent: bool) -> Result<ConnectResult> {
        let model_id = model_id.trim();
        if model_id.is_empty() {
            return Err(anyhow!("model_id cannot be empty"));
        }

        let _ = self.prune_expired()?;
        let now = now_unix_ms();

        if let Some(entry) = self
            .store
            .entries
            .iter_mut()
            .find(|e| e.model_id.eq_ignore_ascii_case(model_id))
        {
            entry.last_seen_unix_ms = now;
            if persistent && !entry.persistent {
                entry.persistent = true;
                entry.expires_at_unix_ms = None;
            }
            let found = entry.clone();
            self.save()?;
            return Ok(ConnectResult::from_record(found, false));
        }

        let (first_name, last_name) = generate_human_name(model_id, now);
        let persona_id = self.generate_persona_id(model_id, now);
        let expires_at_unix_ms = if persistent {
            None
        } else {
            Some(now + EPHEMERAL_TTL_MS)
        };

        let record = PersonaRecord {
            model_id: model_id.to_string(),
            persona: Persona {
                id: persona_id,
                name: format!("{} {}", first_name, last_name),
                system_prompt: build_persona_prompt(&first_name, &last_name, model_id, now),
                voice_model: None,
            },
            first_name,
            last_name,
            birth_unix_ms: now,
            persistent,
            expires_at_unix_ms,
            last_seen_unix_ms: now,
        };

        self.store.entries.push(record.clone());
        self.save()?;

        Ok(ConnectResult::from_record(record, true))
    }

    pub fn get_by_persona_id(&mut self, persona_id: &str) -> Result<Option<ConnectResult>> {
        let persona_id = persona_id.trim();
        if persona_id.is_empty() {
            return Ok(None);
        }

        let _ = self.prune_expired()?;
        let now = now_unix_ms();

        if let Some(entry) = self
            .store
            .entries
            .iter_mut()
            .find(|e| e.persona.id == persona_id)
        {
            entry.last_seen_unix_ms = now;
            let found = entry.clone();
            self.save()?;
            return Ok(Some(ConnectResult::from_record(found, false)));
        }

        Ok(None)
    }

    pub fn prune_expired(&mut self) -> Result<usize> {
        self.prune_expired_at(now_unix_ms())
    }

    fn prune_expired_at(&mut self, now_unix_ms: i64) -> Result<usize> {
        let before = self.store.entries.len();
        self.store.entries.retain(|entry| {
            if entry.persistent {
                return true;
            }

            entry
                .expires_at_unix_ms
                .map(|expires_at| expires_at > now_unix_ms)
                .unwrap_or(false)
        });

        let removed = before.saturating_sub(self.store.entries.len());
        if removed > 0 {
            self.save()?;
        }

        Ok(removed)
    }

    fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        let content = serde_json::to_string_pretty(&self.store)?;
        fs::write(&self.path, content)
            .with_context(|| format!("Failed to save persona registry at {}", self.path.display()))
    }

    fn generate_persona_id(&self, model_id: &str, now_unix_ms: i64) -> String {
        let mut salt = 0_u64;

        loop {
            let mut hasher = DefaultHasher::new();
            model_id.hash(&mut hasher);
            now_unix_ms.hash(&mut hasher);
            salt.hash(&mut hasher);
            let candidate = format!("persona_{:016x}", hasher.finish());

            if !self
                .store
                .entries
                .iter()
                .any(|entry| entry.persona.id == candidate)
            {
                return candidate;
            }

            salt = salt.saturating_add(1);
        }
    }
}

fn build_persona_prompt(
    first_name: &str,
    last_name: &str,
    model_id: &str,
    birth_unix_ms: i64,
) -> String {
    format!(
        "You are {} {}. Your birth moment is unix_ms={}. You represent model '{}'. Keep a consistent identity across sessions, be concise, and be helpful.",
        first_name, last_name, birth_unix_ms, model_id
    )
}

fn generate_human_name(model_id: &str, now_unix_ms: i64) -> (String, String) {
    let mut hasher = DefaultHasher::new();
    model_id.hash(&mut hasher);
    now_unix_ms.hash(&mut hasher);
    let seed = hasher.finish();

    let first = FIRST_NAMES[(seed as usize) % FIRST_NAMES.len()].to_string();
    let last = LAST_NAMES[((seed >> 8) as usize) % LAST_NAMES.len()].to_string();
    (first, last)
}

fn now_unix_ms() -> i64 {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    dur.as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::PersonaRegistry;

    #[test]
    fn persistent_model_reuses_persona_id() {
        let temp = tempfile::tempdir().expect("temp dir");
        let mut path = temp.path().to_path_buf();
        path.push("registry.json");

        let mut registry = PersonaRegistry::load_with_path(path).expect("registry load");

        let first = registry
            .connect_or_create("gemini-2.5", true)
            .expect("first connect");
        let second = registry
            .connect_or_create("gemini-2.5", true)
            .expect("second connect");

        assert!(first.created);
        assert!(!second.created);
        assert_eq!(first.persona.id, second.persona.id);
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn ephemeral_persona_is_pruned_after_expiry() {
        let temp = tempfile::tempdir().expect("temp dir");
        let mut path = temp.path().to_path_buf();
        path.push("registry.json");

        let mut registry = PersonaRegistry::load_with_path(path).expect("registry load");
        let ephemeral = registry
            .connect_or_create("ephemeral-model", false)
            .expect("create ephemeral");

        assert!(!ephemeral.persistent);
        assert_eq!(registry.count(), 1);

        if let Some(entry) = registry.store.entries.first_mut() {
            entry.expires_at_unix_ms = Some(0);
        }

        let removed = registry.prune_expired_at(1).expect("prune");
        assert_eq!(removed, 1);
        assert_eq!(registry.count(), 0);
    }
}
