# AIR Codegen (Phase 2)

Status: **Cranelift IR MVP** for the `sum`-class subset. Interpreter remains the general execution path (`airc run`). No object / executable emit yet.

## Goal

Lower a typechecked air-format AST to a **native** artifact for `std` / hosted first, then freestanding profiles.

## Backend choice

| Backend | Pros | Cons | Decision |
|---------|------|------|----------|
| **Cranelift** | Pure-Rust ecosystem, embeddable, fast compile, good for compiler prototypes | Less mature AOT / fewer targets than LLVM | **MVP backend** |
| LLVM | Mature opts, broad targets | Heavy build, C++/cmake coupling | Later / optional |
| Custom | Full control | Too much work for Phase 2 | Reject for now |

**Backend:** `cranelift-codegen` / `cranelift-frontend` / `cranelift-native` in `crates/airc`.

## Pipeline

```text
.air / .airb
   │ parse + typecheck + ownership (existing)
   ▼
typed AST
   │ airc compile  (Phase 2)
   ▼
Cranelift IR  →  host ISA machcode (in-memory)
                 →  object / executable (later)
```

`airc run` stays the AST interpreter. `airc compile` is the native path; it must refuse ill-typed / ownership-failing modules (same checker as `check`).

## Supported subset (v0)

**In**

- `fn` with `i32` params / `i32` return
- `lit` / `var` / `seq` / `let` / `set!` / `if` / `loop` / `break` / `return`
- Builtin calls: `+ - * /` and `<= < > >= == !=` on `i32` (wrapping arith, matching the interpreter)
- Host ISA: verify Cranelift IR, then `Context::compile` (no `.o` / link yet)

**Out (later)**

- `cap.*`, user fn calls, `match`, arrays, `struct` / `enum`, strings
- Object files / linking a hosted binary
- Freestanding `_start`
- Full LLVM path; generics / traits; tasks / channels

Unsupported constructs yield `codegen.unsupported`. Cranelift failures yield `codegen.error`.

## Phase 2 MVP scope (remaining)

- Emit a relocatable object or link a tiny hosted binary that returns `main`’s `i32`
- Freestanding profile: document “no `cap.print`”; compile may error on hosted caps

## CLI

```bash
airc compile <file.air|.airb> [--diag=text|json]
```

Typechecks, then lowers supported modules (e.g. `examples/sum.air`) through Cranelift.  
`-o` / object emit is still future work.

## Open questions

1. Object format: `*.o` + system linker vs Cranelift JIT demo that runs `main`?
2. Hosted runtime for `cap.print`: thin `libc`/`std` glue vs keep interpreter-only for I/O in MVP?
3. Freestanding first binary: `main`-less `_start` sketch vs hosted-only MVP?
