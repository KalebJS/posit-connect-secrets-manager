# posit-connect-secrets-manager

A terminal UI for bulk-managing environment variables across [Posit Connect](https://posit.co/products/enterprise/connect/) content items.

![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## What it does

Posit Connect stores environment variables per content item. Managing them one at a time through the web UI is tedious at scale. This tool lets you:

- **Browse** all your content items and their environment variables
- **Maintain a local vault** of key/value pairs (stored as JSON)
- **Safe-merge sync** (`Ctrl+U`) — pushes vault values into each project's env vars without touching keys the vault doesn't know about and without adding new keys to projects that don't already define them
- **Filter** any list with fuzzy search
- **Blacklist** projects or individual variables from sync
- **Edit values in `$EDITOR`** for multi-line or sensitive content

## Install

```bash
uv tool install posit-connect-secrets-manager
```

Or from source (requires Rust toolchain):

```bash
uv tool install git+https://github.com/KalebJS/posit-connect-secrets-manager
# or
cargo install --git https://github.com/KalebJS/posit-connect-secrets-manager
```

## Getting started

Launch the TUI:

```bash
posit-secrets
```

<!-- screenshot -->

On first launch you'll land on the **Projects** page. Use `j`/`k` in the sidebar to navigate to **Settings**, then press `l` or `Enter` to enter the settings pane and fill in your server URL and API key. Press `h` or `Esc` to return to the sidebar, then `Ctrl+P` to fetch your content items — they'll populate the project list within a few seconds.

Once projects are loaded, navigate to **Env Vars** in the sidebar to see an aggregated list of every environment variable key across all your projects, alongside the matching value from your vault. Use **Vault** to manage those vault entries. When your vault is ready, press `Ctrl+U` from anywhere to sync vault values into every project that already has a matching key.

## Pages

| Page | Description |
|------|-------------|
| **Projects** | Browse content items and their env vars; expand to inspect, add, delete, or blacklist individual vars |
| **Env Vars** | Aggregated view of all env var keys across projects, showing which have vault values |
| **Vault** | Manage the local key/value store that gets synced to projects |
| **Settings** | Server URL, API key, vault path, and theme |

## Configuration

Config is saved to `~/.config/posit-secrets/config.toml`.

| Field | Description |
|-------|-------------|
| Server URL | Your Posit Connect base URL (e.g. `https://connect.example.com`) |
| API Key | A Posit Connect API key with write access to your content |
| Vault Path | Path to the local JSON vault file (default: `~/.config/posit-secrets/vault.json`) |
| Theme | Color theme — cycle with `Enter` on the theme row |

**Corporate TLS:** set `SSL_CERT_FILE` (path to a PEM file) or `SSL_CERT_DIR` (directory of `*.pem` files) to inject custom CA certificates.

## Keybindings

### Global

| Key | Action |
|-----|--------|
| `Tab` | Toggle sidebar / content focus |
| `q` / `Ctrl+C` | Quit |
| `Ctrl+P` | Refresh project list from Posit Connect |
| `Ctrl+U` | Safe-merge sync vault → all projects (shows confirmation) |

### Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `h` / `←` / `Esc` | Return to sidebar |
| `l` / `→` / `Enter` | Enter content pane (from sidebar) |
| `g` | Jump to top |
| `G` | Jump to bottom |
| `f` / `/` | Open fuzzy filter |
| `F` | Clear filter |

### Projects page

| Key | Action |
|-----|--------|
| `Enter` / `Space` | Expand / collapse project to show its env vars |
| `x` | On a project row: toggle project in/out of sync whitelist. On a var row: toggle var exclusion for that project |
| `a` | Add an env var to the selected project (fuzzy-picks from vault keys) |
| `d` | Delete the selected env var from the project |

### Env Vars page

| Key | Action |
|-----|--------|
| `Enter` / `Space` | Open popup showing which projects use the selected var |
| `e` / `E` | Open the selected var's vault value in `$EDITOR` |

### Vault page

| Key | Action |
|-----|--------|
| `n` | New entry (opens key field for editing) |
| `e` / `Enter` | Edit value of selected entry |
| `E` | Open value in `$EDITOR` |
| `d` / `Delete` | Delete selected entry |

### Settings page

| Key | Action |
|-----|--------|
| `e` / `Enter` | Edit selected field (theme field cycles values instead) |
| `Esc` | Cancel edit |

## Safe-merge sync

`Ctrl+U` iterates every non-blacklisted content item and, for each one:

1. Fetches current env vars from the Connect API
2. Overlays vault values **only for keys that already exist** in the project
3. PATCHes the merged set back

It never adds new environment variables to a project that doesn't already define them, and never deletes existing ones. Safe to run repeatedly.

## License

MIT
