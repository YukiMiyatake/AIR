# AIR

**AI Intermediate Representation** — a **statically typed, systems-capable general-purpose language** with an AI-first canonical AST.

AIR targets C/C++/Rust-class software, including **freestanding / kernel** profiles. Precise widths (`i32`, `i64`, `f64`, …), **no GC** (ownership + borrowing + explicit allocators/arenas), dual lowering (interpreter for bring-up, **native codegen** for production).

Human text syntax is not the source of truth; agents and tools emit typed AST. A mnemonic view exists for inspection.

AIR is unrelated to CostGate / MCP tooling. Communication IR is a separate deferred track.

## Status

Design documentation (**design v0.1 systems**). Toolchain not implemented yet. Example sketches still lag the static/systems model.

## Docs

| Doc | Contents |
|-----|----------|
| [docs/VISION.md](docs/VISION.md) | Why AIR exists; systems + AI-first goals |
| [docs/DESIGN.md](docs/DESIGN.md) | Types, memory, profiles, lowering, traits/vtables |
| [docs/AI_NATIVE.md](docs/AI_NATIVE.md) | Memory, errors, process/shell, capabilities, DI/mocks |
| [docs/AIR_FORMAT.md](docs/AIR_FORMAT.md) | Minimal typed AST schema (air-format v0) |
| [docs/OWNERSHIP.md](docs/OWNERSHIP.md) | v0 ownership, `set!`, lexical borrows |
| [docs/ABSTRACTION.md](docs/ABSTRACTION.md) | Capability vs trait vs vtable layering |
| [docs/SUBSET.md](docs/SUBSET.md) | Phase 1 bootstrap in/out cut |
| [docs/PHASE1_DECISIONS.md](docs/PHASE1_DECISIONS.md) | Overflow, main exit, diagnostics, interpreter |
| [docs/CONCURRENCY.md](docs/CONCURRENCY.md) | Hosted tasks/channels + Alloc |
| [docs/EXAMPLES.md](docs/EXAMPLES.md) | air-format v0 example suite |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Typed bootstrap → native/freestanding → GP growth |

## Goals

- Systems-capable general-purpose (including kernel/freestanding)
- Static typing with precise numeric and pointer types
- Explicit memory: ownership/borrow + allocators/arenas (no default GC)
- AI-first canonical AST and token density
- Native compilation path; interpreter only for bring-up

## Contributing

PR-only to `main` (no `develop`, no direct merges). See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

TBD
