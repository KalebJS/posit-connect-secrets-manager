# posit-connect-secrets-manager

A terminal UI for bulk-managing environment variables (secrets) across [Posit Connect](https://posit.co/products/enterprise/connect/) content items.

![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## What it does

Posit Connect stores environment variables per content item. Managing them one at a time through the web UI is tedious at scale. This tool lets you:

- **Browse** all your content items in a sidebar
- **Maintain a local vault** of key/value pairs (stored as JSON)
- **Safe-merge sync** (`Ctrl+U`) — pushes vault values into each project's env vars without touching keys the vault doesn't know about and without adding new keys to projects that don't already have them

## Install

```bash
git clone https://github.com/kalebsmith/posit-connect-secrets-manager
cd posit-connect-secrets-manager
cargo build --release
# binary at target/release/posit-secrets
```

Or run directly:

```bash
cargo run
```

## Configuration

On first run, press `s` to open the Settings page and enter:

| Field | Description |
|-------|-------------|
| Server URL | Your Posit Connect base URL (e.g. `https://connect.example.com`) |
| API Key | A Posit Connect API key with write access to your content |
| Vault Path | Path to the local JSON vault file (default: `~/.config/posit-secrets/vault.json`) |

Config is saved to `~/.config/posit-secrets/config.toml`.

## Usage

```
Tab           toggle sidebar / content focus
↑ / ↓         navigate list
n             new vault entry
Enter         confirm edit
Esc           cancel edit / back
Ctrl+R        refresh project list from Posit Connect
Ctrl+U        safe-merge sync vault → all projects
q             quit
```

### Safe-merge sync

`Ctrl+U` iterates every content item and, for each one:

1. Fetches current env vars from the Connect API
2. Overlays vault values **only for keys that already exist** in the project
3. PATCHes the merged set back

It never adds new environment variables to a project that doesn't already define them, and never deletes existing ones. Safe to run repeatedly.

## Architecture

Single-binary Rust TUI. The UI thread never blocks — all HTTP calls run in `tokio::spawn` tasks and send results back via an `mpsc` channel.

```
src/
  main.rs          event loop (terminal events + app events + tick)
  app.rs           App struct, all state, keyboard handlers
  config.rs        TOML config at ~/.config/posit-secrets/config.toml
  vault.rs         order-preserving IndexMap vault backed by JSON
  api/client.rs    reqwest wrapper for Posit Connect API
  ui/mod.rs        render() dispatcher
  ui/theme.rs      color/style constants
```

**Posit Connect API calls used:**

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/api/v1/content` | List user's content items |
| `GET` | `/api/v1/content/{guid}/environment` | Read env vars |
| `PATCH` | `/api/v1/content/{guid}/environment` | Write env vars |

Auth: `Authorization: Key <api_key>` header.

## License

MIT
