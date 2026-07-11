# AIR Design

Fixed design decisions for AIR as a **general-purpose language** whose canonical form is an execution IR. Changing these requires a roadmap version bump and an explicit design note.

Status: design-only (no VM implementation yet).

**Product identity:** general-purpose (applications, libraries, concurrent programs). AI-first AST / mnemonic / token density are the representation strategy — not a limit on program size or domain.

**MVP vs target:** early phases ship a **bootstrap subset**. Omissions in the subset are delivery order, not “AIR does not need this.”

## Pipeline

```
Author (agent or tool)  →  Canonical AST  →  Bytecode  →  AIR VM  →  Host / capabilities
                              ↓
                        Mnemonic view (inspect / debug)
```

- Authors read and write **canonical AST** (agents directly; humans via tools or mnemonic round-trip).
- Humans primarily inspect **mnemonic**.
- The VM runs **bytecode** encoded from AST.
- Effectful I/O and OS facilities go through **host / capability** calls.

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
- Entry via `main` (MVP: arity 0; later: args / runtime init).
- **Closures:** required for a general-purpose AIR. **Bootstrap (Phase 1):** top-level `fn` only, no free-variable capture. **Target:** first-class closures / lambdas with upvalues (see [ROADMAP.md](ROADMAP.md)).
- **Concurrency:** required for a general-purpose AIR when throughput and latency matter. **Bootstrap:** single task, deterministic given host replies. **Target model:** lightweight concurrent tasks (**goroutine-like** M:N scheduling) plus **channels** for communication; shared-memory mutation across tasks is restricted until a memory model is specified. Fairness and scheduling details land with the concurrency milestone — not “host-only parallelism forever.”

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

## Modules and linkage

- **Bootstrap:** single module per run.
- **Target:** multi-file modules, imports, and a package story suitable for general-purpose codebases.

## Builtins

**Bootstrap set** (enough to run the example suite and bring up the VM):

- Arithmetic / compare as above
- `len`, `get`, `set` (list/map), `push`
- `str` coercion for print paths
- Result helpers: construct/match `ok` / `err`

**Target:** a real standard library (collections algorithms, strings, concurrency primitives, I/O wrappers over capabilities) — grown deliberately, still AST-friendly and token-dense. LINQ-style query comprehension sugar is optional later; library APIs can cover much of that ground first.

## Paradigm (target)

- **Primary:** imperative, expression-oriented, procedure/function based (not class-OOP).
- **Functional features:** first-class functions and closures are in scope; purity is not mandated.
- **OOP:** no user classes/prototypes in early design; composition via `map` / modules / functions. Revisit only with a strong need.
- **Errors:** result tags remain the default; a future `try`-like sugar must desugar to results (no silent unwind culture).

## Explicit non-goals (representation / product)

- Human sugar syntax as the **canonical** form
- Required interop transpile to Python/JS
- Class-based OOP as the core model (unless later redesign)

## Deferred engineering (not “unwanted”)

- Closures / upvalues (after bootstrap VM)
- Lightweight tasks + channels (after closures/modules foundation)
- JIT / production GC
- Optional static checking
- Rich standard library

## Versioning

- This document is **design v0**.
- Incompatible changes: update this file, note the change in [ROADMAP.md](ROADMAP.md), and bump an eventual `air-format` version when bytecode ships.
