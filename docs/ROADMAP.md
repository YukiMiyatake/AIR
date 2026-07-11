# AIR Roadmap

Phased plan toward a **statically typed, systems-capable** AIR (userland + freestanding/kernel). Early phases bootstrap a subset; native + freestanding are first-class goals, not afterthoughts.

## Phase 0 — Docs (current)

- [VISION.md](VISION.md) — systems GP + AI-first AST
- [DESIGN.md](DESIGN.md) — static types, ownership+allocators, profiles
- [AI_NATIVE.md](AI_NATIVE.md) — errors, process/shell, capabilities, concurrency defaults
- [EXAMPLES.md](EXAMPLES.md) — **needs rewrite** for static/systems model
- This roadmap

## Phase 1 — Typed bootstrap

- Typed AST schema (`i32`/`i64`/`f64`/…, structs, `Result`)
- Typechecker (no execution of ill-typed programs)
- Ownership/move + lexical borrows (minimal lifetime system)
- Explicit `Alloc` / arena parameters for any heap use
- Interpreter for typed subset (bring-up only)
- Rewrite example suite to static types

Exit criteria: ill-typed and obvious UAF/move errors fail at check time; freestanding-shaped examples use only explicit allocators.

## Phase 2 — Native + freestanding

- Native codegen path (backend TBD: LLVM / Cranelift / custom)
- `freestanding` profile: no GC, no hosted I/O runtime
- Target intrinsics sketch (atomics, volatile MMIO, asm) behind `unsafe`
- Agent loop: generate → type/borrow diagnostics → patch

Exit criteria: at least one freestanding binary (e.g. bare demo or kernel module sketch) built without hosted runtime.

## Phase 3 — Language core growth

- Modules / imports
- Generics (monomorphization)
- **Traits / interfaces** for abstraction
- **Explicit vtable / function-record** pattern for DI, mocks, and freestanding drivers
- Closures under ownership rules
- Richer stdlib for `std` profile (collections on allocators)
- Deeper borrow/lifetime expressiveness as needed
- Optional later: `dyn`-like trait objects (fat pointer) as sugar over explicit vtables

## Phase 4 — Concurrency

- Hosted: lightweight tasks + channels
- Freestanding: documented atomics / interrupt model
- Synchronization types

## Phase 5 — Communication IR (separate)

Deferred; do not conflate with the systems language core.

## Versioning

| Label | Meaning |
|-------|---------|
| design v0.1 systems | Static types, no GC, kernel/freestanding goal |
| air-format v0 | First frozen typed AST when Phase 1 schema stabilizes |
