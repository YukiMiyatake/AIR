# AIR

**AI Intermediate Representation** — a **general-purpose language** with an AI-first canonical execution IR and VM.

Authors (agents and tools) emit an explicit AST; a mnemonic view supports human inspection; a VM runs bytecode with effectful operations behind a capability / host boundary. Human text syntax is secondary; machine clarity and token density are primary.

AIR is **not** a short-script DSL. Bootstrap releases may ship a language subset; the product target is general-purpose (modules, closures, concurrency, libraries).

AIR is **not** related to CostGate or MCP tooling. Communication / multi-agent IR is a separate deferred track.

## Status

Design documentation (design v0). Language, bytecode, and VM are not implemented yet.

## Docs

| Doc | Contents |
|-----|----------|
| [docs/VISION.md](docs/VISION.md) | Why AIR exists, audience, non-goals, success metrics |
| [docs/DESIGN.md](docs/DESIGN.md) | Execution model, AST, mnemonic, bytecode, host API |
| [docs/EXAMPLES.md](docs/EXAMPLES.md) | AST / mnemonic / bytecode sketches |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Bootstrap → GP core → concurrency |

## Goals

- General-purpose programs, not glue-only scripts
- Optimize representation for AI generation and understanding, not human sugar syntax
- Minimize tokens per unit of meaning
- Treat programs as explicit AST / IR, not text-first syntax
- Provide a mnemonic view for humans (assembly-like), separate from the canonical form
- Ship a VM that runs AIR directly, then grow modules, closures, and concurrency

## Contributing

PR-only to `main` (no `develop`, no direct merges). See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

TBD
