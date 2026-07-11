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
- [TOOLING.md](TOOLING.md) — Docker-first; Rust primary, TS oracle
- [CONCURRENCY.md](CONCURRENCY.md) — hosted tasks/channels + Alloc (Phase 4)
- [AI_NATIVE.md](AI_NATIVE.md) — errors, process/shell, capabilities, concurrency defaults
- [EXAMPLES.md](EXAMPLES.md) — air-format v0 example suite
- [ENCODING.md](ENCODING.md) — S-expr / JSON / binary layers; user names vs syntax tags
- [CODEGEN.md](CODEGEN.md) — Phase 2 native path sketch
- This roadmap

## Phase 1 — Typed bootstrap

Contract: [SUBSET.md](SUBSET.md). Pre-decisions: [PHASE1_DECISIONS.md](PHASE1_DECISIONS.md). Tooling: [TOOLING.md](TOOLING.md). Encoding: [ENCODING.md](ENCODING.md).

- **[AIR_FORMAT.md](AIR_FORMAT.md) air-format v0** — minimal typed AST
- Typechecker + ownership check ([OWNERSHIP.md](OWNERSHIP.md))
- **AST interpreter** (no bytecode in Phase 1)
- TypeScript **oracle** suite under `tools/airc` (bootstrap; not the primary CLI)
- Overflow / `main` exit / JSON diagnostics per PHASE1_DECISIONS
- Example suite in [EXAMPLES.md](EXAMPLES.md)
- **Hosted stdout** via `cap.print` (required for example / smoke tests — not optional polish)
- **Docker** (`Dockerfile`, `compose.yaml`) as the supported dev environment
- **Encodings:** **canonical S-expr** (`.air`) is the default for edit/PR/agents; JSON (`.air.json`) remains as legacy parity; `airc fmt`; binary pack sketch

Exit criteria: see SUBSET definition of done + PHASE1_DECISIONS CLI behavior.  
In particular: `examples/hello.air` prints `hello` on program stdout (asserted in tests), distinct from the CLI printing the `main` return value.

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
| Done | Docs/CLI: `.air` default; JSON legacy |
| Done | `check` / `run` accept `.airb` |
| Later | Richer binary payloads; drop `.air.json` when TS CLI is retired |

### Hosted I/O priority (do not wait for Phase 3/5)

| Need | When | Notes |
|------|------|--------|
| `cap.print` → stdout | **Phase 1 / 1.5** | Example suite, agent smoke loops |
| stdin / stderr stubs | Phase 1.5+ as needed for tests | Still capability-gated |
| `cap.fs` / richer std I/O | Phase 3 hosted stdlib | After language core growth starts |
| `cap.net` / TCP | after Phase 3 | Policy in [AI_NATIVE.md](AI_NATIVE.md) |
| HTTP / RPC / Communication IR | **Phase 5** | Separate from language core |

## Phase 1.5 — Rust `airc` parity (**met**)

| Item | Status |
|------|--------|
| Parse / check / run in `crates/airc` | Done |
| `fmt` / `hash` / `eq` / `pack` / `unpack`; `.airb` load | Done |
| Single binary via `Dockerfile.airc` (`airc-rs`) | Done |
| `cap.print` stdout parity (`hello.air`) | Done |
| Docker-first workflow | Done |
| TS suite kept as **oracle** (`npm test`) | Done (ongoing) |
| Deprecate / remove TS CLI from default docs | Later |

Exit criteria (met):

- `docker compose run --rm airc-rs version` works  
- `check` / `run` on `examples/sum.air` → **55** (Rust; TS oracle matches)  
- `hello.air` program stdout is `hello` under both CLIs  

**Primary toolchain is Rust.** Agents and contributors should use `cargo run -p airc` / `airc-rs`. See [TOOLING.md](TOOLING.md).

## Phase 2 — Native + freestanding

- [CODEGEN.md](CODEGEN.md) — **Cranelift** backend; `sum`-class IR + JIT-run `main`
- Native codegen path **in Rust airc** (Cranelift MVP first; LLVM optional later)
- `freestanding` profile: no GC, no hosted I/O runtime (`cap.print` is hosted-only)
- Target intrinsics sketch (atomics, volatile MMIO, asm) behind `unsafe`
- Agent loop: generate → type/borrow diagnostics → patch

Exit criteria: at least one freestanding binary (e.g. bare demo or kernel module sketch) built without hosted runtime.

Near-term:

| Step | Deliverable |
|------|-------------|
| Done | CODEGEN.md + `airc compile` typechecks then stubs |
| Done | First Cranelift IR for `sum`-class `i32`/`loop` subset |
| Done | JIT-run parameterless `main` (`sum` → 55) |
| Next | Link hosted binary / object emit; freestanding `_start` sketch |

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
