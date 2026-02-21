# LLM Identity Tool

A lightweight, local, API-less service for assigning and persisting identities for Large Language Models (LLMs) when they connect to tools, agents, or APIs.

## Why this exists?

When interacting with non-persistent LLMs (or APIs), it's useful to provide the model with a consistent persona across different sessions and tools. This tool acts as a local registry:

- Any model (identified by `model_id`) can connect.
- If it's the model's first time connecting, a unique Persona is generated (with a human first name, last name, and birth time).
- Reconnecting with the same `model_id` returns the same exact Persona and history context.
- Personas created for non-persistent LLMs are automatically cleaned up after 1 week to avoid cluttering local storage.
- All storage is local (`~/.kaispeech_personas.json`). **No external API is required.**

## Getting Started

1. **Run the server**

   ```bash
   cargo run
   ```

   The server listens on `0.0.0.0:9005`.

2. **Connect an Agent**

   ```bash
   curl -X POST http://127.0.0.1:9005/agent/connect \
     -H "Content-Type: application/json" \
     -d "{\"model_id\":\"llm://your-model\",\"persistent\":true}"
   ```

   **Response:**
   ```json
   {
     "persona_id": "persona_1234567890abcdef",
     "persona_name": "Aiden Harper",
     "model_id": "llm://your-model",
     "first_name": "Aiden",
     "last_name": "Harper",
     "birth_unix_ms": 1718912345678,
     "persistent": true,
     "expires_at_unix_ms": null,
     "created": true,
     "system_prompt": "You are Aiden Harper. Your birth moment is unix_ms=1718912345678. You represent model 'llm://your-model'. Keep a consistent identity across sessions, be concise, and be helpful."
   }
   ```

3. **Check Health / Active Registrations**

   ```bash
   curl http://127.0.0.1:9005/health
   ```

## Integration

Because it's just a local REST endpoint, any script, MCP Server, or LLM-based application can ping this service on startup to fetch its injected "system prompt" and persona ID before proceeding.
