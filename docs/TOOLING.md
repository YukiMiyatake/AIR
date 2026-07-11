# AIR tooling strategy

How we build and run the AIR toolchain. **Docker is the supported development environment.**

## Implementation languages

| Stage | Language | Path | Role |
|-------|----------|------|------|
| **Production / primary** | **Rust** | `crates/airc/` | Parse, check, run, fmt, hash, pack; future native codegen |
| **Oracle (Phase 1 suite)** | **TypeScript / Node** | `tools/airc/` | Spec regression; keep until TS CLI is retired |
| AIR programs | AIR (air-format) | `examples/*.air` | Not the host language |

Rationale:

- Rust is the Phase 1.5+ host: single binary, Docker image `airc-rs`, systems-aligned (no GC in the product language).
- TS bootstrapped the JSON/S-expr suite quickly; it remains an **oracle** (`npm test`) and optional CLI, not the default agent/dev path.
- Go remains a rejected default for production `airc` (GC runtime).

**Default CLI:** Rust `airc` (`cargo run -p airc` or `docker compose run --rm airc-rs`).  
TS CLI deprecation (remove `tools/airc` bin from docs/CI smoke) is a later cleanup once the oracle is no longer needed.

## Docker-first workflow

Images:

| Image / target | Purpose |
|----------------|---------|
| `air-dev` (`Dockerfile`) | Dev container: Rust + Node + repo mount |
| `airc` (`Dockerfile.airc`) | Release-style image with Rust `airc` binary (`airc-rs` service) |

```bash
# Build dev image
docker compose build dev

# Shell in container (cwd = /workspace)
docker compose run --rm dev bash

# Primary: Rust airc
docker compose run --rm dev cargo run -p airc -- run examples/sum.air
docker compose run --rm dev cargo test --workspace
docker compose run --rm airc-rs version
docker compose run --rm airc-rs run examples/sum.air

# Oracle: TypeScript suite (optional for local; CI still runs it)
docker compose run --rm dev npm ci
docker compose run --rm dev npm test
```

See [compose.yaml](../compose.yaml) and [docs/ROADMAP.md](ROADMAP.md) Phase 1.5.
