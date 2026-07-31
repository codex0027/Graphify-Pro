# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 0.3.x   | ✅ Active          |

## Reporting a Vulnerability

**Do not open a public issue.** Email security concerns to the maintainers.

## Security Design

Graphify Pro processes local source code only. It does not:
- Send code to external services (fully offline)
- Execute or evaluate source code
- Access network resources during extraction

### Built-in Protections

- **File size caps**: Files exceeding `--max-file-size` (default 10MB) are skipped
- **Path traversal prevention**: All file paths are resolved relative to the project root; `../` escapes are rejected
- **Graph size enforcement**: Graph JSON files exceeding safety limits are rejected before parsing
- **Secure defaults**: Extraction runs with the user's permissions only; no elevated access needed
- **Atomic writes**: Output files are written atomically to prevent corruption on crash or concurrent access
- **Metadata sanitization**: Node labels/IDs are sanitized to prevent injection

### Dependencies

All dependencies are pinned via `Cargo.lock`. Run `cargo audit` periodically to check for known vulnerabilities:

```bash
cargo install cargo-audit
cargo audit
```
