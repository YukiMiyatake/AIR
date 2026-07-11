# AIR

**AI Intermediate Representation** — a **statically typed, systems-capable general-purpose language** with an AI-first canonical AST.

AIR targets C/C++/Rust-class software, including **freestanding / kernel** profiles. Precise widths (`i32`, `i64`, `f64`, …), **no GC** (ownership + borrowing + explicit allocators/arenas), dual lowering (interpreter for bring-up, **native codegen** for production).

Human text syntax is not the source of truth; agents and tools emit typed AST. A mnemonic view exists for inspection.

AIR is unrelated to CostGate / MCP tooling. Communication IR is a separate deferred track.

## Status

- Design docs + Phase 1 **TypeScript** `tools/airc` (parse/check/run; `examples/sum.air.json` → 55)
- **Rust** `crates/airc` scaffold (production path)
- **Docker-first** development ([docs/TOOLING.md](docs/TOOLING.md))

## Docker (supported workflow)

```bash
docker compose build dev
docker compose run --rm dev bash

# TypeScript bootstrap airc
docker compose run --rm dev npm ci
docker compose run --rm dev npm run airc -- run examples/sum.air.json

# Rust airc scaffold
docker compose run --rm dev cargo test --workspace
docker compose run --rm dev cargo run -p airc -- version

# Release-style binary image
docker compose build airc-rs
docker compose run --rm airc-rs version
```

## Local (optional)

Node 22+ for TS; Rust 1.85+ for `crates/airc`. Prefer Docker if toolchains differ.

```bash
npm ci && npm test && npm run airc -- run examples/sum.air.json
cargo test --workspace && cargo run -p airc -- version
```

## Docs

| Doc | Contents |
|-----|----------|
| [docs/TOOLING.md](docs/TOOLING.md) | Docker-first; TS → Rust host language plan |
| [docs/VISION.md](docs/VISION.md) | Why AIR exists; systems + AI-first goals |
| [docs/DESIGN.md](docs/DESIGN.md) | Types, memory, profiles, lowering, traits/vtables |
| [docs/AI_NATIVE.md](docs/AI_NATIVE.md) | Memory, errors, process/shell, capabilities, DI/mocks |
| [docs/AIR_FORMAT.md](docs/AIR_FORMAT.md) | Minimal typed AST schema (air-format v0) |
| [docs/ENCODING.md](docs/ENCODING.md) | S-expr / JSON / binary; Diff and user-type rules |
| [docs/OWNERSHIP.md](docs/OWNERSHIP.md) | v0 ownership, `set!`, lexical borrows |
| [docs/ABSTRACTION.md](docs/ABSTRACTION.md) | Capability vs trait vs vtable layering |
| [docs/SUBSET.md](docs/SUBSET.md) | Phase 1 bootstrap in/out cut |
| [docs/PHASE1_DECISIONS.md](docs/PHASE1_DECISIONS.md) | Overflow, main exit, diagnostics, interpreter |
| [docs/CONCURRENCY.md](docs/CONCURRENCY.md) | Hosted tasks/channels + Alloc |
| [docs/EXAMPLES.md](docs/EXAMPLES.md) | air-format v0 example suite |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Bootstrap → Rust parity → native/freestanding |

## Goals

- Systems-capable general-purpose (including kernel/freestanding)
- Static typing with precise numeric and pointer types
- Explicit memory: ownership/borrow + allocators/arenas (no default GC)
- AI-first canonical AST and token density
- Native compilation path; interpreter only for bring-up

## Contributing

PR-only to `main` (no `develop`, no direct merges). See [CONTRIBUTING.md](CONTRIBUTING.md). Prefer Docker for builds.

## License

TBD
