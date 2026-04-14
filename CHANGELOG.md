## 0.3.2 (2026-04-14)

### Fix

- load corporate CA bundle from SSL_CERT_FILE/SSL_CERT_DIR

## 0.3.1 (2026-04-14)

### Fix

- **env-vars**: handle string-array response and per-project error state
- revert reqwest to rustls-tls (cross-compilation)

## 0.3.0 (2026-04-14)

### Fix

- revert reqwest to rustls-tls (fix cross‑compilation)

### Feat

- **projects**: filter list to owner/editor roles only

## 0.2.2 (2026-04-14)

### Fix

- switch reqwest from rustls to native-tls

## 0.2.1 (2026-04-14)

### Fix

- use __api__ path prefix for all Posit Connect endpoints
- remove pypi support

## 0.2.0 (2026-04-14)

### Feat

- add uv tool install support via maturin
