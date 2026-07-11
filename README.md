# AIR

**AI Intermediate Representation** — an AI-first **execution IR** and small VM.

Agents emit and edit an explicit AST; a mnemonic view exists for human inspection; a deterministic VM runs bytecode with side effects only through a permissioned host API. Human readability is secondary; machine clarity and token density are primary.

AIR is **not** related to CostGate or MCP tooling. Communication / multi-agent IR is deferred.

## Status

Design documentation (design v0). Language, bytecode, and VM are not implemented yet.

## Docs

| Doc | Contents |
|-----|----------|
| [docs/VISION.md](docs/VISION.md) | Why AIR exists, audience, non-goals, success metrics |
| [docs/DESIGN.md](docs/DESIGN.md) | Execution model, AST, mnemonic, bytecode, host API |
| [docs/EXAMPLES.md](docs/EXAMPLES.md) | AST / mnemonic / bytecode sketches |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Phases (execution IR first; communication IR later) |

## Goals

- Optimize for AI generation and understanding, not human ergonomics
- Minimize tokens per unit of meaning
- Treat programs as explicit AST / IR, not text-first syntax
- Provide a mnemonic view for humans (assembly-like), separate from the canonical form
- Ship a small VM that runs AIR directly

## Contributing

PR-only to `main` (no `develop`, no direct merges). See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

TBD
