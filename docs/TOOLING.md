# AIR tooling strategy

How we build and run the AIR toolchain. **Docker is the supported development environment.**

## Implementation languages

| Stage | Language | Path | Role |
|-------|----------|------|------|
| Phase 1 bootstrap (current) | **TypeScript / Node** | `tools/airc/` | Spec validation: parse / check / run (`.air` preferred; `.air.json` legacy) |
| Production toolchain | **Rust** | `crates/airc/` | Speed, single binary, native codegen path |
| AIR programs themselves | AIR (air-format) | `examples/*.air` | Not the host language |

Rationale:

- TS was chosen only to bootstrap quickly (JSON-native, Node available). It is **not** the long-term host.
- Rust aligns with AIR’s systems goals (no GC in the product language, freestanding/native later) and ships a real binary.
- Go remains a rejected default for the production `airc` (GC runtime); may still appear in peripheral scripts if needed.

Migration: feature-parity in Rust (`check` / `run` / diagnostics), then deprecate TS CLI (keep as oracle tests until parity).

## Docker-first workflow

Images:

| Image / target | Purpose |
|----------------|---------|
| `air-dev` (`Dockerfile`) | Dev container: Rust + Node + repo mount |
| `airc` (`Dockerfile.airc`) | Release-style image with Rust `airc` binary |

```bash
# Build dev image
docker compose build dev

# Shell in container (cwd = /workspace)
docker compose run --rm dev bash

# Rust airc (preferred)
docker compose run --rm dev cargo run -p airc -- run examples/sum.air

# TypeScript airc (bootstrap)
docker compose run --rm dev npm ci
docker compose run --rm dev npm run airc -- run examples/sum.air

# Rust tests
docker compose run --rm dev cargo test --workspace
docker compose run --rm dev cargo run -p airc -- version
```

See [compose.yaml](../compose.yaml) and [docs/ROADMAP.md](ROADMAP.md) Phase 1.5.
