# AIR Design

Design decisions for AIR as a **statically typed, systems-capable general-purpose language** with an AI-first canonical AST.

Status: design-only. Changing these requires a roadmap note.

**Product identity:** C/C++/Rust-class scope (including **kernel / freestanding**), AI-first representation.

**Bootstrap vs target:** early toolchains may interpret a subset; omissions are delivery order.

## Pipeline

```
Author (agent/tool) → Typed canonical AST → Type + ownership check
                         ↓                      ↓
                   Mnemonic view          Bytecode interp (bring-up)
                                                ↓
                                          Native codegen (production / freestanding)
```

- Canonical form is always the **typed AST**.
- **Interpreter/VM** is for bring-up, tests, and agent loops — not the only runtime story.
- **Native compilation** is required for performance and freestanding/kernel deployment.

## Profiles

| Profile | Intent |
|---------|--------|
| `std` / hosted | Userland: richer capability surface (files, net, threads via runtime) |
| `freestanding` | No hosted OS assumptions: no GC, no implicit alloc, explicit allocator args, minimal runtime |

Kernel code uses `freestanding` (plus target-specific privileged ops as explicit intrinsics/capabilities).

**AI-Native defaults** for memory, errors, process/shell, capabilities, and concurrency are normative in [AI_NATIVE.md](AI_NATIVE.md). This file states the type/runtime skeleton; that file states the effect and safety policy.

## Type system (static)

Dynamic typing is **out**. Programs do not typecheck → do not run / do not codegen.

### Primitive types (initial set)

| Type | Meaning |
|------|---------|
| `bool` | Boolean |
| `i8` `i16` `i32` `i64` | Signed integers |
| `u8` `u16` `u32` `u64` | Unsigned integers |
| `f32` `f64` | IEEE-754 floats |
| `usize` `isize` | Pointer-sized ints (target-dependent) |
| `str` | UTF-8 string (**hosted**; freestanding prefers byte arrays/slices) |
| `never` | Diverging (optional early) |

There is **no** single `num` type. Arithmetic requires matching widths or **explicit casts**.
Literal and operator rules: [AIR_FORMAT.md](AIR_FORMAT.md) § Literals / Arithmetic.
### Pointers and references (target)

| Form | Meaning |
|------|---------|
| `*T` / raw ptr | Freestanding / unsafe escape (restricted) |
| `&T` | Shared borrow |
| `&mut T` | Unique mutable borrow |
| owned `T` | Value / owning place (move semantics for non-`Copy`) |

`Copy` types: primitives and explicitly marked POD aggregates.

### Aggregates

- `struct` / product types with named fields (typed)
- `enum` / sum types (including `Result[T, E]`-style)
- Arrays `[T; N]` and slices `&[T]` / `&mut [T]`
- **No** GC’d open `list` / `map` as language primitives. Collections are libraries built on allocators (or fixed buffers in freestanding)

### Generics

Phased: monomorphic bootstrap first; type parameters next; full trait/bound system later. Kernel code must remain monomorphizable (no mandatory runtime reflection).

## Memory management (no GC)

**GC is not part of AIR’s default runtime.** Kernel and freestanding profiles forbid it.

### Chosen model (hybrid)

AIR adopts a **hybrid** of Rust-like ownership and Zig-like explicit allocation:

1. **Ownership + move** by default for non-`Copy` values (affine/linear discipline).
2. **Borrowing** — shared (`&`) and unique (`&mut`) with non-aliasing rules (Rust-like).
3. **Explicit allocators / arenas** — any heap growth takes an `Alloc` (or arena) parameter; no global hidden heap.
4. **Region / arena scopes** — bump arenas for request-, IRQ-, or phase-scoped memory; free all at once (kernel-friendly).

Rationale vs pure alternatives: see “Memory options considered” below.

### Unsafe

An explicit `unsafe` (or capability) boundary is required for:

- raw pointer deref
- bypassing borrow rules
- inline asm / privileged register ops
- volatile MMIO

Agents should prefer safe AST; unsafe nodes are auditable and rare.

### Memory options considered

| Approach | Pros | Cons | Verdict for AIR |
|----------|------|------|-----------------|
| **GC** | Easy authoring | Latency, freestanding/kernel hostile | **Reject** as default |
| **Manual malloc/free (C)** | Simple model | UAF/double-free; bad for AI authors | Reject as default |
| **Rust ownership + full lifetimes** | Proven systems safety | Lifetime polymorphism is hard for LLMs; steep diagnostics | **Adopt core**; phase full lifetime generics |
| **Zig-style explicit allocators** | Excellent freestanding/kernel fit; honest costs | Alone does not prevent aliasing bugs | **Adopt** together with ownership |
| **Arena / region-only** | Simple, fast, kernel-friendly | Awkward for long-lived graph structures | **Adopt** as primary freestanding pattern |
| **Linear types only (no borrows)** | Simpler checker | Clumsy APIs; lots of threading-through | Too limited alone |

**Why not “Rust only”?** Full lifetime inference/elaboration is a major AI failure mode. AIR should start with **ownership + lexical/region borrows**, then add richer lifetime parameters as the checker and agent loop mature.

**Why not “Zig only”?** Kernel *and* AI-authored code need stronger aliasing/ownership static checks than allocator passing alone provides.

**Operational rules (v0):** [OWNERSHIP.md](OWNERSHIP.md) — moves, `set!`, lexical borrows, error codes.

## Error model

No C++-style exception unwind. Fallible APIs use **`Result[T, E]`** (or equivalent tagged ok/err). Optional propagate sugar desugars to `Result`. Abort/panic is explicit and profile-restricted.

Normative policy: [AI_NATIVE.md](AI_NATIVE.md) § Errors and “exceptions”.

## Execution / lowering

### Bootstrap interpreter

Stack + locals bytecode for a **typed** subset (bring-up, tests). Not the kernel deployment path.

### Native compilation (required path)

- Lower typed AST → native (via custom backend and/or LLVM/Cranelift — choice deferred).
- Freestanding: no runtime GC, optional tiny panic/abort stub, caller-supplied allocators only.
- Hosted: thin runtime for capabilities (files, net, scheduling) implemented outside freestanding core.

## Concurrency (target)

- Lightweight tasks + channels remain the high-level hosted model; **channel buffers come from an explicit Alloc** ([CONCURRENCY.md](CONCURRENCY.md)).
- Freestanding/kernel: explicit threads/ISRs/atomics as **target intrinsics**; no mandatory M:N runtime in freestanding.
- Shared mutability across tasks requires `unsafe` or typed synchronization (mutex/atomics).
- Not in Phase 1 — see [SUBSET.md](SUBSET.md).
## Control forms (AST)

Normative shapes for the bootstrap subset live in [AIR_FORMAT.md](AIR_FORMAT.md) (**air-format v0**). Summary:

| Form | Role |
|------|------|
| `seq` / `let` / `set!` / `var` | Sequencing and locals |
| `if` / `loop` / `break` / `return` / `call` | Control and calls |
| `match` | Exhaustive split (`Result`, enums) |
| `as` / `borrow` / `move` | Casts and ownership hooks |
| `cap` | Hosted effects (replaces ad-hoc dynamic `host`) |
| `lit` / `array_lit` / `struct_lit` | Typed literals |

Do not invent alternate tag spellings in examples; extend **air-format** instead.
## Mnemonic view

Unchanged requirements: one op per line, explicit slots, **round-trip** with AST. Mnemonics should surface types at function boundaries.

## Paradigm

- Imperative, expression-oriented, procedure/function based
- First-class functions/closures **where allocation/ownership allows** (closures that capture may require allocator or stack-only rules)
- Not class-OOP; not prototype-OOP (see Abstraction below)
- Systems: explicit memory, static types, freestanding profile

## Abstraction, DI, and mocks

AIR needs **substitutable abstractions** for dependency injection and test doubles. It does **not** need classes or JavaScript-style prototypes.

| Mechanism | Status | Role |
|-----------|--------|------|
| Class hierarchy / inheritance | **Rejected** as core model | Hidden coupling; poor AST clarity |
| JS-style prototypes | **Rejected** | Runtime delegation mutation; hostile to static checking and audit |
| Traits / interfaces (Go/Rust-like) | **Adopt** | Named capability sets for APIs (`Fs`, `Clock`, …) |
| Explicit DI (pass deps as args) | **Adopt** | No ambient service locator / global singleton as default |
| Explicit vtable / function record | **Adopt** | Runtime swap for mocks, drivers, freestanding ops bundles |
| Static dispatch (monomorphized traits) | **Adopt** | Zero-cost when the impl is known at compile time |
| Dynamic trait objects (`dyn`-like) | **Optional later** | Fat pointer + hidden vtable when type erasure is required |

### Explicit vtable (normative pattern)

A vtable here means a **visible struct of function pointers** (and optional context pointer), not an invisible object-system table:

```text
struct FileOps {
  ctx: *mut (),
  read:  fn(*mut (), buf: &mut [u8]) -> Result[usize, IoError],
  write: fn(*mut (), buf: &[u8]) -> Result[usize, IoError],
}
```

- Production code and tests pass different `FileOps` values.
- Kernel/freestanding drivers naturally look like this.
- The AST shows which ops bundle is used — AI-Native and auditable.

**Do not reject vtables.** Reject only *implicit* class/prototype dispatch that agents cannot see.

**Layering with capabilities:** [ABSTRACTION.md](ABSTRACTION.md).

### DI policy

- Prefer **constructor-/argument-injection** of traits, ops structs, or capabilities.
- Mocks are alternate impls or alternate vtables — not monkey-patching prototypes.
- Global mutable registries for services are non-goals.

## Explicit non-goals

- Dynamic typing / single `num`
- Default GC runtime
- Human sugar as canonical form
- Class-based OOP as the core model
- Prototype-based OOP (JS-style)

## Versioning

- This document supersedes earlier dynamic-typed sketches (**design v0.1 systems**).
- Abstraction/DI/vtable policy added for mocks and freestanding drivers.
- Example programs in [EXAMPLES.md](EXAMPLES.md) still reflect the old dynamic sketch and must be rewritten.
