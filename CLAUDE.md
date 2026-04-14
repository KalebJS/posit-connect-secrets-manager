# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo build                   # compile
cargo run                     # run the TUI app (binary: posit-secrets)
cargo check                   # fast type-check without producing a binary
cargo clippy                  # lint
```

## Architecture

Single-binary Rust TUI app. All state lives in `App` (`src/app.rs`). The `main.rs` event loop drives everything via a `tokio::select!` over three sources:

1. **Terminal events** — `crossterm::EventStream` → `App::handle_crossterm_event`
2. **Background task results** — `mpsc::Receiver<AppEvent>` → `App::handle_app_event`
3. **Tick timer** (250ms) → `App::on_tick` (spinner animation, status message expiry)

Heavy work (HTTP calls to Posit Connect) always happens in `tokio::spawn` tasks that send `AppEvent` variants back over the channel — the UI thread never blocks.

### Key modules

| Module | Role |
|--------|------|
| `src/app.rs` | `App` struct, all UI state, keyboard handlers, background task launchers (`trigger_fetch`, `trigger_sync`) |
| `src/api/client.rs` | `ConnectClient` — thin `reqwest` wrapper; `list_content`, `get_env_vars`, `set_env_vars` |
| `src/vault.rs` | `Vault` — `IndexMap<String,String>` backed by a local JSON file; order-preserving |
| `src/config.rs` | `Config` — TOML file at `~/.config/posit-secrets/config.toml` |
| `src/ui/mod.rs` | Top-level `render()` dispatcher; splits frame into sidebar / content / status bar |
| `src/ui/theme.rs` | All `Color` and `Style` constants (sky-blue + orange palette) |

### Posit Connect API

- **Auth header**: `Authorization: Key <api_key>`
- `GET /__api__/v1/content` → list user's content items
- `GET /__api__/v1/content/{guid}/environment` → `Vec<EnvVar>`
- `PATCH /__api__/v1/content/{guid}/environment` → full replacement; code always safe-merges (fetches current vars first, overlays vault values) before PATCHing

### Navigation model

`App::sidebar_focused: bool` determines whether keystrokes go to the sidebar (page switching) or the active page's content handler. `Tab` toggles focus. Each page has its own handler in `App` (`handle_project_list_key`, `handle_vault_key`, etc.).

### Vault editing flow

New entry (`n`): inserts `("", "")` into `IndexMap`, starts editing the Key field. On Enter, replaces the empty key with the typed key and automatically transitions to editing the Value field. On Esc, removes the empty-key placeholder.

### Safe-merge sync (Ctrl+U)

For each project: fetch its current env vars, overlay vault values for keys that already exist (never adds new keys, never deletes existing ones), then PATCH the merged set.
