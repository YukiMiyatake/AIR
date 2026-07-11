# AIR Roadmap

Phased plan toward a **general-purpose** AIR. Early phases are a **bootstrap subset**, not a permanent “scripting-only” product.

Communication / multi-agent IR remains a separate later track.

## Phase 0 — Docs (current)

- [VISION.md](VISION.md) — general-purpose + AI-first representation
- [DESIGN.md](DESIGN.md) — execution model; bootstrap vs target
- [EXAMPLES.md](EXAMPLES.md) — paired AST / mnemonic / bytecode sketches
- This roadmap

Deliverable: design baseline (design v0). No runtime yet.

## Phase 1 — Bootstrap VM

Minimum executable core (subset of the language):

- Parse / validate canonical AST
- Compile AST → bytecode
- Execute `main`: arithmetic, compare, `seq` / `if` / `loop` / `break` / `return` / `call` / `set!`
- Locals via slots; lists + `len` / `get` / `set` / `push`
- Result tags `ok` / `err` (no exceptions)
- Stub host/capabilities: at least `print`; allowlist + audit log
- Mnemonic projector with round-trip tests on the example suite
- Single task only (concurrency not in this phase)

Exit criteria: examples in [EXAMPLES.md](EXAMPLES.md) run; capability calls appear in an audit log.

## Phase 2 — Agent + developer loop

- “Generate/edit AST → run → structured diagnostics → patch” cycle
- Stable machine-readable diagnostics
- Token-density measurement on a suite that includes **non-trivial** programs (not only micro-scripts)

Exit criteria: an agent or scripted stand-in can iterate to a passing run without human syntax repair.

## Phase 3 — General-purpose language core

Language features expected of a real GP language:

- Multi-file modules / imports
- **Closures and lambdas** (upvalues)
- Richer standard library surface (still capability-backed I/O)
- Optional static checks (not a full static-typing product yet)

Exit criteria: express common library patterns without encoding everything as indexed loops + top-level fns only.

## Phase 4 — Concurrency and performance

Required for GP workloads where speed and latency matter:

- **Lightweight tasks** (goroutine-like) with **M:N** scheduling
- **Channels** (and select-style receive) as the primary cross-task communication
- Memory / sharing rules documented (start restrictive)
- Runtime performance work as needed (allocator, later JIT/GC productization)

Exit criteria: parallel CPU- and latency-sensitive examples are expressible **in AIR**, not only by shelling out to a host “run this thread pool” escape hatch.

## Phase 5 — Communication IR (deferred, separate)

Separate design document when started. Not part of the execution language core.

Until then: do not conflate multi-agent protocol design with the VM.

## Versioning

| Label | Meaning |
|-------|---------|
| design v0 | Current docs (GP identity clarified) |
| air-format v0 | First frozen AST + bytecode when Phase 1 ships |

Incompatible design changes before `air-format v0`: update DESIGN, note here, keep examples aligned.
