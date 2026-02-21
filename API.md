# API Documentation

## `POST /agent/connect`

Creates or resumes a Persona for an LLM connection.

### Request Body

```json
{
  "model_id": "string",
  "persistent": true
}
```

- `model_id` (string, required): A unique string identifying the model (e.g., `gemini-1.5-pro` or `llm://local-llama`).
- `persistent` (boolean, optional, default=true): Whether this Persona should be kept indefinitely. If `false`, the persona expires 7 days after the last connection and is automatically purged.

### Response

```json
{
  "persona_id": "string",
  "persona_name": "string",
  "model_id": "string",
  "first_name": "string",
  "last_name": "string",
  "birth_unix_ms": 0,
  "persistent": true,
  "expires_at_unix_ms": null,
  "created": true,
  "system_prompt": "string"
}
```

- `persona_id` (string): The unique, deterministic identifier assigned to this persona for the given model_id.
- `created` (boolean): `true` if this is the first time the model_id has connected.
- `system_prompt` (string): A pre-generated prompt you can inject into the LLM system prompt so it understands its identity.

---

## `GET /health`

Returns server health and statistics.

### Response

```json
{
  "status": "ok",
  "registry_personas": 5
}
```
