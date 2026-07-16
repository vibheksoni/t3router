## Learned User Preferences

- Default chat model preference is `gemini-2.5-flash-lite` for low latency
- Kimi preference is `kimi-k2.5` (latest Kimi); there is no `kimi-2.6-flash` slug on t3.chat
- System prompt should be set at the beginning of a chat session (via `T3_SYSTEM_PROMPT` in `.env`)
- Prefer `T3_TRACK_CREDITS=false` for snappy responses (skip balance API)

## Learned Workspace Facts

- t3router is a Rust library for interacting with t3.chat
- Authentication uses browser session cookies (`COOKIES` + `CONVEX_SESSION_ID` in `.env`), not API keys; requires a paid t3.chat account
- Interactive terminal chat: `cargo run --bin t3chat` (or `cargo run --example chat`); REPL lives in `src/t3/repl.rs`
- `T3_MODEL`, `T3_SYSTEM_PROMPT`, `T3_TIMEZONE`, `T3_LOCALE`, `T3_TRACK_CREDITS` configure chat
- `T3_TRACK_CREDITS=false` skips balance API calls; when enabled, credit polling runs in background after streaming
- Sessions auto-save to `~/.t3router/session.json`; `/resume` reloads; `Client::resume_conversation()` for programmatic resume
- Streaming via `send_stream()` / `send_with_credits_stream()`; SSE parsed by `SseAccumulator` in `client.rs`
- Client refactored with `build_chat_body()` and `warmup()` (lighter startup than full `init()`)
- Latest Kimi model on t3.chat is `kimi-k2.5`; `ling-2.6-flash` is a separate 2.6 Flash model (InclusionAI, not Kimi)
- Codebase memory indexed as project `Users-marius-t3router` (graph at `.codebase-memory/graph.db.zst`)
