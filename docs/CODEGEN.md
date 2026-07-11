# AIR Codegen (Phase 2)

Status: **Cranelift hosted + freestanding MVP** for the `sum`-class subset — JIT-run `main`, emit `.o`, link hosted (`cc`) or freestanding (`_start` + `-nostdlib -static`) binaries. Interpreter remains the general execution path (`airc run`).

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
   └─ ObjectModule → .o
         ├─ hosted:     cc → binary (CRT)
         └─ freestanding: assemble _start.S → cc -nostdlib -static -no-pie
```

`airc run` stays the AST interpreter. `airc compile` is the native path; it must refuse ill-typed / ownership-failing modules (same checker as `check`).

## Supported subset (v0)

**In**

- `fn` with `i32` params / `i32` return
- `lit` / `var` / `seq` / `let` / `set!` / `if` / `loop` / `break` / `return`
- Builtin calls: `+ - * /` and `<= < > >= == !=` on `i32` (wrapping arith, matching the interpreter)
- JIT-run parameterless `main`
- `-o file.o` → relocatable object; other `-o path` → hosted binary via `cc`
- `--freestanding -o path` → Linux x86_64 / aarch64 `_start` (call `main`, `SYS_exit`) + static link without libc
- Hosted `cap.print` of **string literals** via libc `puts` (rejected under `--freestanding`)

**Out (later)**

- `cap.print` of non-literal strings / richer caps; user fn calls, `match`, arrays, `struct` / `enum`
- Freestanding on non-Linux hosts
- Full LLVM path; generics / traits; tasks / channels

Unsupported constructs yield `codegen.unsupported`. Cranelift failures yield `codegen.error`. Freestanding misuse yields `codegen.freestanding`.

## Freestanding `_start`

Sources live under `crates/airc/runtime/`:

| Host | File | Exit |
|------|------|------|
| Linux x86_64 | `linux-x86_64/_start.S` | `SYS_exit` (60) |
| Linux aarch64 | `linux-aarch64/_start.S` | `SYS_exit` (93) |

Behavior:

1. Require parameterless `main () i32` (same as JIT demo).
2. Reject `cap.*` via the existing MVP subset (no hosted I/O in freestanding artifacts).
3. Assemble CRT0 with `cc -c`, then:
   - executable: `cc -nostdlib -static -no-pie -o out air.o _start.o`
   - object: `cc -nostdlib -r -o out.o air.o _start.o`

`--freestanding` without `-o` is an error.

## CLI

```bash
airc compile <file.air|.airb> [--diag=text|json]
airc compile <file.air|.airb> -o out.o
airc compile <file.air|.airb> -o out
airc compile <file.air|.airb> --freestanding -o out
```

Examples:

```text
ok: compiled module sum (jit main => 55)
ok: compiled module sum (jit main => 55) -> /tmp/sum
ok: compiled module sum (jit main => 55) [freestanding] -> /tmp/sum-free
```

## Open questions

1. ~~Object format~~ — `.o` + system `cc` for hosted binaries (done)
2. ~~Hosted `cap.print`~~ — libc `puts` for string literals (done); richer values later
3. ~~Freestanding `_start`~~ — Linux x86_64 / aarch64 done; other targets later
