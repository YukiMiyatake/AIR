# AIR Codegen (Phase 2 sketch)

Status: **design sketch** — no object code yet. Interpreter remains the execution path (`airc run`).

## Goal

Lower a typechecked air-format AST to a **native** artifact for `std` / hosted first, then freestanding profiles.

## Backend choice (tentative)

| Backend | Pros | Cons | Decision |
|---------|------|------|----------|
| **Cranelift** | Pure-Rust ecosystem, embeddable, fast compile, good for compiler prototypes | Less mature AOT / fewer targets than LLVM | **Adopt for MVP** |
| LLVM | Mature opts, broad targets | Heavy build, C++/cmake coupling | Later / optional |
| Custom | Full control | Too much work for Phase 2 | Reject for now |

**v0 backend: Cranelift** (via `cranelift-*` crates in `crates/airc` when implementation starts).

## Pipeline

```text
.air / .airb
   │ parse + typecheck + ownership (existing)
   ▼
typed AST
   │ airc compile  (Phase 2)
   ▼
Cranelift IR  →  object / executable
```

`airc run` stays the AST interpreter. `airc compile` is the native path; it must refuse ill-typed / ownership-failing modules (same checker as `check`).

## Phase 2 MVP scope

**In**

- Hosted `main() -> i32` subset already covered by examples (`i32` arith, `if`/`loop`, `let`/`set!`, fixed arrays, `Result`/`match`, `struct`/`enum`, `cap.print` stub or link to hosted runtime)
- Emit a relocatable object or link a tiny hosted binary that returns `main`’s `i32`
- Freestanding profile: document “no `cap.print`”; compile may error on hosted caps

**Out (later)**

- Full LLVM path
- Generics / traits (Phase 3)
- Tasks / channels (Phase 4)
- Richer `.airb` as the sole compile input (optional convenience)

## CLI (sketch)

```bash
airc compile <file.air|.airb> [-o out]
```

Today: typechecks, then exits with a stable “not implemented” diagnostic (`codegen.unimplemented`).  
Does **not** invent IR yet.

## Open questions

1. Object format: `*.o` + system linker vs Cranelift JIT-only demo?
2. Hosted runtime for `cap.print`: thin `libc`/`std` glue vs keep interpreter-only for I/O in MVP?
3. Freestanding first binary: `main`-less `_start` sketch vs hosted-only MVP?

Resolve these when the first Cranelift IR emitter lands.
