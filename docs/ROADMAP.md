# AIR Roadmap

Phased plan toward a **statically typed, systems-capable** AIR (userland + freestanding/kernel). Early phases bootstrap a subset; native + freestanding are first-class goals, not afterthoughts.

## Phase 0 — Docs (current)

- [VISION.md](VISION.md) — systems GP + AI-first AST
- [DESIGN.md](DESIGN.md) — static types, ownership+allocators, profiles
- [AIR_FORMAT.md](AIR_FORMAT.md) — **air-format v0** minimal typed AST
- [OWNERSHIP.md](OWNERSHIP.md) — v0 move/`set!`/borrow operational rules
- [ABSTRACTION.md](ABSTRACTION.md) — capability vs trait vs vtable
- [SUBSET.md](SUBSET.md) — **Phase 1 in/out cut**
- [PHASE1_DECISIONS.md](PHASE1_DECISIONS.md) — overflow, main exit, diagnostics, interpreter
- [CONCURRENCY.md](CONCURRENCY.md) — hosted tasks/channels + Alloc (Phase 4)
- [AI_NATIVE.md](AI_NATIVE.md) — errors, process/shell, capabilities, concurrency defaults
- [EXAMPLES.md](EXAMPLES.md) — air-format v0 example suite
- This roadmap

## Phase 1 — Typed bootstrap

Contract: [SUBSET.md](SUBSET.md). Pre-decisions: [PHASE1_DECISIONS.md](PHASE1_DECISIONS.md).

- **[AIR_FORMAT.md](AIR_FORMAT.md) air-format v0** — minimal typed AST
- Typechecker + ownership check ([OWNERSHIP.md](OWNERSHIP.md))
- **AST interpreter** (no bytecode in Phase 1); TypeScript reference CLI under `tools/airc`
- Overflow / `main` exit / JSON diagnostics per PHASE1_DECISIONS
- Example suite in [EXAMPLES.md](EXAMPLES.md)

Exit criteria: see SUBSET definition of done + PHASE1_DECISIONS CLI behavior.

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

- Hosted: lightweight tasks + channels per [CONCURRENCY.md](CONCURRENCY.md) (queues via explicit Alloc)
- Freestanding: documented atomics / interrupt model
- Synchronization types
- Optional deterministic scheduler seed for replay tests

## Phase 5 — Communication IR (separate)

Deferred; do not conflate with the systems language core.

## Versioning

| Label | Meaning |
|-------|---------|
| design v0.1 systems | Static types, no GC, kernel/freestanding goal |
| air-format v0 | Minimal typed AST draft — see [AIR_FORMAT.md](AIR_FORMAT.md) |
