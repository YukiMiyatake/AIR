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
- [TOOLING.md](TOOLING.md) — Docker-first; TS bootstrap → Rust production
- [CONCURRENCY.md](CONCURRENCY.md) — hosted tasks/channels + Alloc (Phase 4)
- [AI_NATIVE.md](AI_NATIVE.md) — errors, process/shell, capabilities, concurrency defaults
- [EXAMPLES.md](EXAMPLES.md) — air-format v0 example suite
- [ENCODING.md](ENCODING.md) — S-expr / JSON / binary layers; user names vs syntax tags
- This roadmap

## Phase 1 — Typed bootstrap

Contract: [SUBSET.md](SUBSET.md). Pre-decisions: [PHASE1_DECISIONS.md](PHASE1_DECISIONS.md). Tooling: [TOOLING.md](TOOLING.md). Encoding: [ENCODING.md](ENCODING.md).

- **[AIR_FORMAT.md](AIR_FORMAT.md) air-format v0** — minimal typed AST
- Typechecker + ownership check ([OWNERSHIP.md](OWNERSHIP.md))
- **AST interpreter** (no bytecode in Phase 1)
- TypeScript reference CLI under `tools/airc` (bootstrap)
- Overflow / `main` exit / JSON diagnostics per PHASE1_DECISIONS
- Example suite in [EXAMPLES.md](EXAMPLES.md)
- **Hosted stdout** via `cap.print` (required for example / smoke tests — not optional polish)
- **Docker** (`Dockerfile`, `compose.yaml`) as the supported dev environment
- **Encodings:** JSON bootstrap + **canonical S-expr** (`.air`); `airc fmt`; binary pack later

Exit criteria: see SUBSET definition of done + PHASE1_DECISIONS CLI behavior.  
In particular: `examples/hello.air.json` prints `hello` on program stdout (asserted in tests), distinct from the CLI printing the `main` return value.

### Encoding track (parallel to language features)

| Step | Deliverable |
|------|-------------|
| Docs | [ENCODING.md](ENCODING.md) — tags closed; user names open; Diff via S-expr |
| Done | Rust `fmt` + `.air` parse; all Phase 1 `examples/*.air` |
| Done | `airc hash` / `airc eq` |
| Done | Line-oriented `fmt` polish |
| Done | TS `.air` parse |
| Done | Binary `.airb` v1 sketch (`pack` / `unpack`) |
| Done | User `struct` + `struct_lit` + `field` |
| Done | User `enum` + `variant_lit` + `variant` match |
| Done | Tuple enum payloads (`[ty...]`) |
| Done | Field store builtin `fset` |
| Later | Richer binary; deprecate JSON as default |

### Hosted I/O priority (do not wait for Phase 3/5)

| Need | When | Notes |
|------|------|--------|
| `cap.print` → stdout | **Phase 1 / 1.5** | Example suite, agent smoke loops |
| stdin / stderr stubs | Phase 1.5+ as needed for tests | Still capability-gated |
| `cap.fs` / richer std I/O | Phase 3 hosted stdlib | After language core growth starts |
| `cap.net` / TCP | after Phase 3 | Policy in [AI_NATIVE.md](AI_NATIVE.md) |
| HTTP / RPC / Communication IR | **Phase 5** | Separate from language core |

## Phase 1.5 — Rust `airc` parity

- Port parse / check / run to `crates/airc` (Rust)
- Ship single binary via `Dockerfile.airc`
- Keep TS suite as oracle until parity; then deprecate TS CLI
- Dev workflow remains Docker-first
- Parity includes **`cap.print` stdout** (same lines as TS for `hello.air.json`)

Exit criteria: `docker compose run --rm airc-rs version` works; `check`/`run` on `examples/sum.air.json` matches TS results (**55**); `hello.air.json` stdout is `hello` under both CLIs.

## Phase 2 — Native + freestanding

- Native codegen path (backend TBD: LLVM / Cranelift / custom) **in Rust airc**
- `freestanding` profile: no GC, no hosted I/O runtime (`cap.print` is hosted-only)
- Target intrinsics sketch (atomics, volatile MMIO, asm) behind `unsafe`
- Agent loop: generate → type/borrow diagnostics → patch

Exit criteria: at least one freestanding binary (e.g. bare demo or kernel module sketch) built without hosted runtime.

## Phase 3 — Language core growth

- Modules / imports
- Generics (monomorphization)
- **Traits / interfaces** for abstraction
- **Explicit vtable / function-record** pattern for DI, mocks, and freestanding drivers
- Closures under ownership rules
- Richer stdlib for `std` profile (collections on allocators; **fs / richer I/O caps**)
- Deeper borrow/lifetime expressiveness as needed
- Optional later: `dyn`-like trait objects (fat pointer) as sugar over explicit vtables

## Phase 4 — Concurrency

- Hosted: lightweight tasks + channels per [CONCURRENCY.md](CONCURRENCY.md) (queues via explicit Alloc)
- Freestanding: documented atomics / interrupt model
- Synchronization types
- Optional deterministic scheduler seed for replay tests

## Phase 5 — Communication IR (separate)

TCP/HTTP/RPC-style **communication IR** — deferred; do not conflate with the systems language core or with Phase 1 stdout.

## Versioning

| Label | Meaning |
|-------|---------|
| design v0.1 systems | Static types, no GC, kernel/freestanding goal |
| air-format v0 | Minimal typed AST draft — see [AIR_FORMAT.md](AIR_FORMAT.md) |
