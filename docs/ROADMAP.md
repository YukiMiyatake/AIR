# AIR Roadmap

Phased plan. **Execution IR first.** Communication / multi-agent IR is explicitly later and out of scope for early docs beyond this note.

## Phase 0 — Docs (current)

- [VISION.md](VISION.md) — why AIR exists, non-goals, success metrics
- [DESIGN.md](DESIGN.md) — execution model, AST, mnemonic, bytecode families, host API
- [EXAMPLES.md](EXAMPLES.md) — paired AST / mnemonic / bytecode sketches
- This roadmap

Deliverable: design baseline (design v0). No runtime yet.

## Phase 1 — MVP VM

- Parse / validate canonical AST
- Compile AST → bytecode
- Execute `main`: arithmetic, compare, `seq` / `if` / `loop` / `break` / `return` / `call`
- Locals via slots; `set!` or equivalent stores
- Lists + `len` / `get` / `set` / `push`
- Result tags `ok` / `err` (no exceptions)
- Stub host: at least `print`; allowlist + audit log hook
- Mnemonic projector with round-trip tests on the example suite

Exit criteria: examples in [EXAMPLES.md](EXAMPLES.md) run; host calls appear in an audit log.

## Phase 2 — Agent loop

- Minimal “generate AST → run → structured error/result → patch AST” cycle
- Stable, machine-readable diagnostics (type/host/runtime codes)
- Token-density measurement harness vs Python on a fixed suite ([VISION.md](VISION.md) metrics)

Exit criteria: an agent (or scripted stand-in) can iterate to a passing run without human syntax repair.

## Phase 3 — Language growth (still execution IR)

- Multi-file modules / imports
- Closures or an explicit alternative for higher-order patterns
- Richer host surface under permission policy
- Optional static checks (not full static typing product)

## Phase 4 — Communication IR (deferred)

Separate design document when started. Not part of execution IR v0–v1.

Possible future concerns (placeholder only):

- Compact structured messages between agents (plans, diffs, tool results)
- Whether communication IR shares AST tags with execution IR or is a distinct format

Until Phase 4 starts: **do not** conflate protocol design with the VM.

## Versioning

| Label | Meaning |
|-------|---------|
| design v0 | Current docs |
| air-format v0 | First frozen AST + bytecode when Phase 1 ships |

Incompatible design changes before `air-format v0`: update DESIGN, note here, keep examples aligned.
