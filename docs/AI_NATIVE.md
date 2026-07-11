# AIR AI-Native Defaults

How AIR chooses **memory, errors, processes, shell, I/O, and concurrency** so the language is native to AI authors (agents) — not merely “inspired by Rust/C++/Go.”

Companion to [DESIGN.md](DESIGN.md) and [VISION.md](VISION.md).

## What “AI Native” means here

Design for agents that **generate, patch, and audit** programs represented as typed AST:

1. **Effects are visible in the AST** — no ambient I/O, no hidden allocation, no silent unwind.
2. **Failures are values** — control flow stays dataflow-friendly for repair loops.
3. **No stringly semantics as the primary API** — structured arguments beat shell one-liners.
4. **Authority is closed by default** — capabilities / profiles; deny unless granted.
5. **Diagnostics are machine-readable** — type, borrow, and capability errors agents can act on.

Human ergonomic sugar is secondary and must desugar into the above.

## Comparison sources (what we take / reject)

| Area | C++ | Go | Rust | AIR choice |
|------|-----|-----|------|------------|
| Memory | new/delete, RAII | GC | ownership + borrow | **Ownership + borrow + explicit Alloc/arena** (no GC) |
| Errors | exceptions / errno | `error` values + panic | `Result` + panic | **`Result[T, E]` as normal path**; no exception mechanism |
| Process | fragmented APIs | structured `os/exec` | structured `std::process` | **Structured spawn** (`argv`, env, cwd, fds) |
| Shell | `system` culture | possible, not primary | possible, not primary | **Forbidden without capability**; last resort only |
| File/net | ambient open | ambient open | mostly ambient | **Capability-gated** (`cap.fs`, `cap.net`, …) |
| Concurrency | threads + shared mem | goroutine + channel (+ GC) | threads/async + ownership | **Hosted: tasks + channels**; **freestanding: explicit low-level** |
| Top-level effects | linkers / static ctors | `init` side effects | more restrained | **No top-level effects**; only `main` + passed caps |
| Abstraction / DI | classes + virtuals | interfaces (not prototypes) | traits (+ `dyn`) | **Traits/interfaces + explicit DI + explicit vtables**; no class/prototype OOP |

AIR is not a clone of any one of these languages. It **re-assembles** the parts that stay explicit under AI authorship.

---

## Memory

**Default: no GC.**

| Rule | Detail |
|------|--------|
| Ownership / move | Non-`Copy` values move; double-use is a check error |
| Borrows | `&T` / `&mut T` with non-aliasing rules |
| Heap | Requires explicit `Alloc` or arena argument — no global hidden heap |
| Arenas / regions | Preferred freestanding pattern (bump, free all at scope end) |
| `unsafe` | Raw pointers, borrow bypass, MMIO, asm — auditable and rare |

**Rejected as default:** tracing GC (Go/Java), ambient `malloc` without ownership (C).

**Why AI Native:** allocation and aliasing show up in types/AST; agents cannot “accidentally” rely on a collector or global heap. Full Rust lifetime polymorphism is phased so early diagnostics stay tractable for models.

See also [DESIGN.md](DESIGN.md) § Memory management.

---

## Errors and “exceptions”

**No C++-style exception unwind as a language feature.**

| Mechanism | Role |
|-----------|------|
| `Result[T, E]` | Normal fallible API return; match / propagate explicitly |
| Propagate sugar | Optional AST sugar (`?`) **desugars to Result** — never to hidden throw |
| Abort / panic | Explicit node; may be **forbidden in `freestanding`** or restricted by profile |
| Errno-style globals | Not used |

**Why AI Native:** repair loops need a value to branch on. Unwinding skips frames the agent did not model and hides effect boundaries.

---

## Process management

**Primary API: structured process spawn** (Go/Rust style, not C `system`).

Conceptual shape (informative):

```text
spawn({
  argv: ["tool", "--flag", path],   // []str, not one shell string
  env:  {...},                      // explicit
  cwd:  path,
  stdin / stdout / stderr: fds or pipes,
  caps: ...                         // subset of parent capabilities
}) -> Result[Child, SpawnError]
```

| Allowed | Forbidden as primary |
|---------|----------------------|
| Argv arrays | Single string interpreted by `/bin/sh` |
| Explicit env / cwd | Implicit parent env mutation as API core |
| Wait / kill / pipes as typed ops | Ad-hoc signal races without types |

Child processes receive a **capability subset** (no ambient inheritance of full parent authority by default).

---

## Shell launch

| Policy | Detail |
|--------|--------|
| Default | **Cannot** invoke a shell |
| Grant | Only with an explicit capability, e.g. `cap.shell` |
| Representation | Even then, prefer structured argv to `sh -c`; `sh -c` is a documented footgun |
| Auditing | Every shell invocation is a first-class effect in the AST / audit log |

**Why AI Native:** models are fluent at shell strings and also fluent at injection and irreversible commands. Closing the shell by default is the highest-leverage safety default for agent-authored systems code.

---

## Files, network, and other I/O

Ambient authority (any function can `open` / `connect`) is **rejected** for hosted AIR.

| Capability (examples) | Grants | Timing |
|-----------------------|--------|--------|
| `cap.print` | Write to hosted stdout | **Phase 1** (examples / smoke) |
| `cap.fs` (scoped) | Path-limited file ops | Phase 3+ |
| `cap.net` | Connect/listen as specified | after Phase 3; HTTP/RPC → Phase 5 |
| `cap.proc` | Structured spawn | after Phase 1 |
| `cap.shell` | Shell (discouraged) | after Phase 1 |
| `cap.time` / `cap.rand` | Clock / entropy | as needed |

- Capabilities are **values** passed into `main` / libraries (or equivalent entry), not ambient globals.
- `freestanding` omits hosted caps; privileged CPU/MMIO ops are separate **target intrinsics** behind `unsafe` / platform caps.

**Why AI Native:** the AST shows what a module *can* do; agents and reviewers can deny caps without reading every callee.

---

## Concurrency

| Profile | Model |
|---------|-------|
| Hosted (`std`) | Lightweight **tasks + channels** (CSP-like; Go-shaped *without* GC) |
| Freestanding / kernel | Explicit threads / ISRs / atomics as **intrinsics**; no mandatory M:N runtime |

Shared mutable state across tasks requires synchronization types or `unsafe`. Ownership rules still apply.

**Allocation:** channel buffers and spawn captures use an explicit **`Alloc`** — see [CONCURRENCY.md](CONCURRENCY.md). Not part of Phase 1 ([SUBSET.md](SUBSET.md)).

**Why AI Native:** channels keep cross-task dataflow explicit; freestanding avoids a hidden scheduler the kernel did not ask for.

---

## Program startup and side effects

- **No** arbitrary top-level / static initializer side effects (reject C++ dynamic init and Go `init` as a pattern).
- Entry is `main` (or freestanding `_start` equivalent) with **explicit arguments + capabilities**.
- Library “constructors” that touch the OS are disallowed; return capabilities or pure values instead.

---

## Abstraction, DI, and mocks

Mocks and dependency injection need **substitutability**, not prototypes.

| Adopt | Reject |
|-------|--------|
| Traits / interfaces | Class inheritance as the core model |
| Explicit DI (pass dependencies) | Ambient service locators / monkey-patching |
| **Explicit vtables** (function-pointer structs) | JS-style prototype chains |
| Monomorphized static dispatch when impl is known | Hidden virtual dispatch as the only style |

### Why not prototypes

JS prototypes mutate shared delegation at runtime. That hides the active implementation from the AST, breaks static reasoning, and is a poor fit for ownership, capabilities, and kernel code.

Go’s strength for testing is **interfaces**, not prototypes. Rust’s is **traits**. AIR follows that family.

### Why vtables are wanted

An **explicit** ops/vtable value is AI-Native:

- Tests swap a mock `FileOps` without patching globals.
- Drivers and freestanding code already look like ops tables.
- Dispatch targets appear in data the agent can see and patch.

Static trait monomorphization covers the zero-cost path; explicit vtables cover runtime substitution. Dynamic trait objects (`dyn`-like fat pointers) may come later as sugar over the same idea.

Normative detail: [DESIGN.md](DESIGN.md) § Abstraction, DI, and mocks.  
**Layering (cap vs trait vs vtable):** [ABSTRACTION.md](ABSTRACTION.md).

---

## Diagnostics (agent loop)

Toolchain outputs should be structured (stable codes), for example:

- `type.mismatch`, `type.width`
- `mem.use_after_move`, `mem.borrow_conflict`, `mem.alloc_missing`
- `cap.missing`, `cap.shell_denied`
- `proc.spawn_invalid`

Agents patch AST from these codes; human prose is secondary.

---

## Summary table (AIR defaults)

| Concern | AI-Native default |
|---------|-------------------|
| Memory | Ownership + borrow + explicit Alloc/arena; **no GC** |
| Exceptions | **None**; `Result` only for normal errors |
| Process | Structured `spawn` + capability subset |
| Shell | **Off** unless `cap.shell` |
| File/net/time/rand | Capability-gated |
| Concurrency | Hosted tasks+channels; freestanding explicit low-level |
| Startup | No ambient top-level OS effects |
| Abstraction / DI | Traits/interfaces + explicit DI + **explicit vtables**; no class/prototype OOP |

## Non-goals of this document

- Exact AST node schemas for every cap (versioned later in `air-format`)
- Full syscall surface for each OS
- Claiming AI Native means “easier than Python” — it means **explicit, auditable, repairable** under agent authorship
