# AIR Design

Fixed design decisions for the **execution IR**. Changing these requires a roadmap version bump and an explicit design note.

Status: design-only (no VM implementation yet).

## Pipeline

```
Agent emits/edits  →  Canonical AST  →  Bytecode  →  AIR VM  →  Host API
                         ↓
                   Mnemonic view (inspect / debug)
```

- Agents read and write **canonical AST**.
- Humans primarily read **mnemonic**.
- The VM runs **bytecode** encoded from AST.
- All I/O goes through **host** calls.

## Value model

Dynamic types (static typing is deferred):

| Tag | Meaning |
|-----|---------|
| `null` | Absence |
| `bool` | `true` / `false` |
| `num` | IEEE-754 float64 (integers are nums with integral value) |
| `str` | UTF-8 string |
| `list` | Ordered sequence of values |
| `map` | String-keyed map of values |
| `fn` | Function reference (index into the module’s function table) |

No user-defined classes or prototypes in v0.

## Error model

There is **no exception / unwind mechanism**.

Fallible operations return a **result** value:

- `["ok", value]`
- `["err", code, message]`

`code` is a short string (e.g. `"div0"`, `"host"`, `"type"`). Agents branch on the tag. Host failures surface as `err` results unless the host aborts the run by policy.

## Execution model

- **Bytecode VM** with an operand **stack** and **local slots** per call frame.
- Programs are a single module with entry function `main` (arity 0 in MVP; args via host later).
- **No closures** in phase 1 — nested functions may exist as top-level `fn` entries only; no free-variable capture.
- Evaluation is single-threaded and deterministic given a fixed sequence of host replies.

### Control forms (AST)

| Form | Shape | Meaning |
|------|-------|---------|
| `seq` | `["seq", e1, e2, ...]` | Evaluate in order; value of last |
| `let` | `["let", [[name, expr], ...], body]` | Bind locals in body |
| `set!` | `["set!", name, expr]` | Store into an existing local slot (mutation; not a host effect) |
| `if` | `["if", cond, then, else]` | Branch; `else` required |
| `loop` | `["loop", body]` | Repeat body until `break` |
| `break` | `["break", value]` | Exit nearest loop with value |
| `return` | `["return", value]` | Return from current function |
| `call` | `["call", callee, arg...]` | Call function or builtin |
| `host` | `["host", op, arg...]` | Host effect |

`set!` on an unbound name is a compile/runtime error. MVP examples prefer `set!` for loops; `let` remains for non-mutating scopes.

Arithmetic and comparisons are builtins invoked via `call` (e.g. `["call", "+", a, b]`).

## Canonical AST

### Encoding

- A program is a JSON (or MessagePack) **array tree**.
- Each compound node is `[tag, ...children]` where `tag` is a string.
- Atoms: JSON `null`, `true`/`false`, numbers, strings.
- Lists/maps as values use tags: `["list", ...]`, `["map", [k, v], ...]`.

**Not canonical:** objects with many named keys (`{"type":"If","cond":...}`). Those may appear as debug dumps but are not the interchange form agents should emit.

### Module shape

```json
[
  "mod",
  ["fn", "main", [], body],
  ["fn", "name", ["a", "b"], body]
]
```

- First function named `main` is the entry (or the sole `fn` tagged entry — MVP requires an explicit `main`).
- Parameter names become local slots 0..n-1 in order.

### Identifier policy

- Prefer short names in agent-generated code (`a`, `xs`, `i`).
- Builtins use fixed short symbols: `+ - * / % < <= > >= == !=` and `list`, `len`, `get`, `set`, `has`.

## Mnemonic view

Requirements:

1. **One instruction (or labeled form) per line.**
2. **Slots explicit** — locals as `s0`, `s1`, … or named aliases declared at function head.
3. **Round-trip** — `AST → mnemonic → AST` and `mnemonic → AST → mnemonic` (normalized) must succeed for valid programs.

Sketch (informative; exact opcode names live with the bytecode table):

```
fn main
  const 0
  store s0
loop L0
  load s0
  const 10
  lt
  jump_if_false L1
  ...
  jump L0
L1:
  load s0
  return
```

Humans may edit mnemonic in emergencies; the **supported** agent path remains AST.

## Bytecode (sketch)

Encoded from AST by a compiler pass. MVP opcode families:

| Family | Examples | Notes |
|--------|----------|-------|
| Constants | `CONST_N`, `CONST_STR`, `CONST_TRUE`, `CONST_NULL` | Pool for strings / large nums |
| Stack / locals | `LOAD`, `STORE`, `POP`, `DUP` | Slot index operand |
| Arithmetic / compare | `ADD`, `SUB`, `MUL`, `DIV`, `MOD`, `CMP_*` | Pop operands, push result |
| Control | `JUMP`, `JUMP_IF`, `JUMP_IF_NOT`, `LOOP` labels | Relative offsets |
| Call | `CALL`, `RET` | Function index + argc |
| Data | `LIST_NEW`, `LIST_GET`, `LIST_SET`, `MAP_*` | Dynamic collections |
| Host | `HOST` | op index + argc; pushes result |
| Result | `OK`, `ERR`, `IS_OK` | Helpers for result tags |

Exact binary layout (endianness, varints) is deferred to implementation; this document fixes the **families** and stack discipline.

## Host API

- Only `["host", op, ...args]` performs effects (filesystem, network, clock, process, randomness, stdout).
- The host implements a **permission allowlist**. Denied ops return `["err", "host", "..."]` or abort per policy.
- The VM must be able to emit an **audit log** of every host call (op, args summary, result tag).
- MVP stub host: `print`, `now` (optional), and no network.

Example:

```json
["host", "print", ["call", "+", "hello ", name]]
```

## Modules and linkage (MVP)

- Single module per run.
- No import / multi-file (phase 2).
- No package registry.

## Builtins (MVP set)

Enough for examples and agent loops:

- Arithmetic / compare as above
- `len`, `get`, `set` (list/map), `push`
- `str` coercion for print paths
- Result helpers: construct/match `ok` / `err`

## Explicit non-goals (design)

- Closures / upvalues (phase 2+)
- Exceptions, `try`/`catch`
- JIT / GC productization
- Human sugar syntax as canonical form
- Required interop transpile to Python/JS

## Versioning

- This document is **design v0**.
- Incompatible changes: update this file, note the change in [ROADMAP.md](ROADMAP.md), and bump an eventual `air-format` version when bytecode ships.
