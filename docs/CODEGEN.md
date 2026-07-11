# AIR Codegen (Phase 2)

Status: **Cranelift hosted MVP** for the `sum`-class subset — JIT-run `main`, emit `.o`, or link a hosted binary with `cc`. Interpreter remains the general execution path (`airc run`).

## Goal

Lower a typechecked air-format AST to a **native** artifact for `std` / hosted first, then freestanding profiles.

## Backend choice

| Backend | Pros | Cons | Decision |
|---------|------|------|----------|
| **Cranelift** | Pure-Rust ecosystem, embeddable, fast compile, good for compiler prototypes | Less mature AOT / fewer targets than LLVM | **MVP backend** |
| LLVM | Mature opts, broad targets | Heavy build, C++/cmake coupling | Later / optional |
| Custom | Full control | Too much work for Phase 2 | Reject for now |

**Backend:** `cranelift-codegen` / `frontend` / `native` / `module` / `jit` / `object` in `crates/airc`.

## Pipeline

```text
.air / .airb
   │ parse + typecheck + ownership (existing)
   ▼
typed AST
   │ airc compile  (Phase 2)
   ▼
Cranelift IR
   ├─ JITModule → call parameterless main() -> i32
   └─ ObjectModule → .o  →  (optional) cc → hosted binary
```

`airc run` stays the AST interpreter. `airc compile` is the native path; it must refuse ill-typed / ownership-failing modules (same checker as `check`).

## Supported subset (v0)

**In**

- `fn` with `i32` params / `i32` return
- `lit` / `var` / `seq` / `let` / `set!` / `if` / `loop` / `break` / `return`
- Builtin calls: `+ - * /` and `<= < > >= == !=` on `i32` (wrapping arith, matching the interpreter)
- JIT-run parameterless `main`
- `-o file.o` → relocatable object; other `-o path` → temp `.o` + `cc -o path`

**Out (later)**

- `cap.*`, user fn calls, `match`, arrays, `struct` / `enum`, strings
- Freestanding `_start` / no-libc link (sketch below)
- Full LLVM path; generics / traits; tasks / channels

Unsupported constructs yield `codegen.unsupported`. Cranelift failures yield `codegen.error`.

## Freestanding `_start` (sketch)

Hosted `main` is a C ABI `() -> i32` symbol linked with the platform CRT via `cc`.

Freestanding (no hosted I/O, no CRT) would instead:

1. Reject `cap.*` at compile time (already true for the MVP subset).
2. Emit a symbol `_start` (or target-specific entry) that:
   - sets up a stack if the loader does not,
   - calls AIR `main` (or an explicit entry fn),
   - exits via a raw syscall / QEMU semihosting / `hlt` loop — **not** `libc` `exit`.
3. Link with `-nostdlib -static` (and a tiny asm/crt0 if required), without depending on `cap.print`.

This PR does **not** implement freestanding linking; the hosted `-o` path remains CRT-based. Exit criterion for Phase 2 freestanding is still open.

## CLI

```bash
airc compile <file.air|.airb> [--diag=text|json]
airc compile <file.air|.airb> -o out.o
airc compile <file.air|.airb> -o out      # hosted binary via cc
```

Examples:

```text
ok: compiled module sum (jit main => 55)
ok: compiled module sum (jit main => 55) -> /tmp/sum.o
ok: compiled module sum (jit main => 55) -> /tmp/sum
```

## Open questions

1. ~~Object format~~ — `.o` + system `cc` for hosted binaries (done)
2. Hosted runtime for `cap.print`: thin `libc`/`std` glue vs keep interpreter-only for I/O in MVP?
3. Freestanding: implement `_start` + `-nostdlib` link next, or keep hosted-only until more of the language lowers?
