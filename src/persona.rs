use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Persona {
    pub id: String,
    pub name: String,
    pub system_prompt: String,
    /// Identifier for the specific TTS voice/model associated with this persona
    pub voice_model: Option<String>,
}

impl Default for Persona {
    fn default() -> Self {
        Self {
            id: "default_assistant".to_string(),
            name: "Kai".to_string(),
            system_prompt: "You are Kai, a helpful, concise AI assistant. You respond via continuous audio stream, so keep your sentences short and natural for speech.".to_string(),
            voice_model: None,
        }
    }
}
