# posit-connect-secrets-manager

A terminal UI for bulk-managing environment variables across [Posit Connect](https://posit.co/products/enterprise/connect/) content items.

![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## What it does

Posit Connect stores environment variables per content item. Managing them one at a time through the web UI is tedious at scale. This tool lets you:

- **Browse** all your content items and their environment variables
- **Maintain a local vault** of key/value pairs (stored as JSON)
- **Safe-merge sync** (`Ctrl+U`) — pushes vault values into each project's env vars without touching keys the vault doesn't know about and without adding new keys to projects that don't already define them
- **Filter projects** with fuzzy search
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

On first launch you'll land on the **Projects** page with an empty sidebar. Use `j`/`k` in the sidebar to navigate to **Settings**, then `l` or `Enter` to enter it and fill in your server URL and API key. Press `Esc` or `h` to return to the sidebar, then `Ctrl+P` to fetch your content items — they'll populate within a few seconds.

Navigate to a project with `j`/`k`, press `l` or `Enter` to move focus into the content pane, then `Enter` or `Space` on a project row to expand its environment variables. Switch to the **Vault** page via the sidebar to define the values you want to sync. When your vault is ready, `Ctrl+U` pushes values to every project that already has matching keys.

## Configuration

On first run, press `s` to open Settings and enter:

| Field | Description |
|-------|-------------|
| Server URL | Your Posit Connect base URL (e.g. `https://connect.example.com`) |
| API Key | A Posit Connect API key with write access to your content |
| Vault Path | Path to the local JSON vault file (default: `~/.config/posit-secrets/vault.json`) |

Config is saved to `~/.config/posit-secrets/config.toml`.

**Corporate TLS:** set `SSL_CERT_FILE` (path to a PEM file) or `SSL_CERT_DIR` (directory of `*.pem` files) to inject custom CA certificates.

## Keybindings

### Global

| Key | Action |
|-----|--------|
| `Tab` | Toggle sidebar / content focus |
| `q` / `Ctrl+C` | Quit |
| `Ctrl+P` | Refresh project list from Posit Connect |
| `Ctrl+U` | Safe-merge sync vault → all projects |

### Navigation (all pages)

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `h` / `←` / `Esc` | Back / go to sidebar |
| `l` / `→` / `Enter` | Enter content pane (from sidebar) |
| `g` | Jump to top |
| `G` | Jump to bottom |

### Projects page

| Key | Action |
|-----|--------|
| `Enter` / `Space` | Expand / collapse project |
| `f` / `/` | Open fuzzy filter |
| `F` | Clear filter |
| `x` | Toggle project sync (whitelist) — or toggle var exclusion when a var is selected |
| `a` | Add env var to selected project |
| `d` | Delete selected env var |

### Vault page

| Key | Action |
|-----|--------|
| `n` | New entry |
| `e` | Edit value |
| `E` | Edit key — or open value in `$EDITOR` when on value field |
| `d` / `Delete` | Delete entry |

### Settings page

| Key | Action |
|-----|--------|
| `e` / `Enter` | Edit selected field |
| `Esc` | Cancel edit |

## Safe-merge sync

`Ctrl+U` iterates every non-blacklisted content item and, for each one:

1. Fetches current env vars from the Connect API
2. Overlays vault values **only for keys that already exist** in the project
3. PATCHes the merged set back

It never adds new environment variables to a project that doesn't already define them, and never deletes existing ones. Safe to run repeatedly.

## License

MIT
