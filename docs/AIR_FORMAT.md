# AIR Format (air-format v0 draft)

Frozen **minimal** typed AST interchange for programs that must be writable and checkable before the full language exists.

Status: **draft v0** — normative for the Phase 1 bootstrap subset. Larger features (generics, traits, full lifetimes, tasks) are out of this document until a later format version.

Encoding: **JSON array trees** (`.air.json`, Phase 1 bootstrap) and **normalized S-expr** (`.air`, canonical text — see [ENCODING.md](ENCODING.md)). Each compound node is `[tag, ...]` / `(tag …)` with the same AST. Binary tagged AST comes later; user-defined names stay in a symbol table, not the tag enum.

## Design goals for v0

1. Enough to write **simple typed programs** (loops, arithmetic, `Result`, fixed arrays, `match`).
2. Unambiguous for agents: every construct has one canonical shape.
3. Compatible with later ownership/capability extensions (holes marked **v0.1+**).

## Module

```json
["mod", name?, item...]
```

- `name` optional string (file/module name).
- `item` is `fn` or `struct` or `enum` (v0).

Entry: a function named `main` must exist for hosted runnable examples.

```json
["fn", "main", params, ret_ty, body]
```

Hosted v0 `main` params (choose one style per program; prefer A):

- **A (recommended):** `[]` and I/O via explicit capability args only when needed later  
- **B:** `[["caps", CapSet]]` when effects are required (shape of `CapSet` is **v0.1+**; v0 examples may omit I/O)

## Types (`ty`)

```text
ty :=
  | "bool" | "i8" | "i16" | "i32" | "i64"
  | "u8" | "u16" | "u32" | "u64"
  | "f32" | "f64" | "usize" | "isize"
  | "str"                         // UTF-8 string value (hosted); see notes
  | "never"
  | ["ptr", ty]                   // *T raw (unsafe contexts only in later checks)
  | ["ref", "shared" | "mut", ty] // &T / &mut T
  | ["array", ty, n]              // [T; N], n is uint literal
  | ["slice", "shared" | "mut", ty]
  | ["fn", [ty...], ret_ty]       // function pointer type
  | ["named", path]               // user struct/enum name (string or ["path", ...])
  | ["result", ty, ty]            // Result[T, E]
```

### `str` (v0 decision)

- `str` is a **first-class type** in hosted profile (UTF-8).
- Freestanding programs should prefer `["array", "u8", N]` / slices; using `str` in `freestanding` is a typecheck error in v0 tooling goals.
- String **literals** use JSON strings with inferred type `str` in hosted code.

## Items

### Function

```json
["fn", name, [[param_name, ty], ...], ret_ty, body]
```

### Struct

```json
["struct", name, [[field_name, ty], ...]]
```

### Enum

```json
["enum", name, variant...]
variant := [variant_name] | [variant_name, ty] | [variant_name, [ty...]]
```

Sugar: `["result", T, E]` is equivalent to a standard enum `Result` with `Ok`/`Err` (implementations may desugar).

## Expressions (`expr`)

### Atoms

| JSON | Rule |
|------|------|
| `null` | Not a value in v0 (use explicit unit later). **Invalid** as expr. |
| `true` / `false` | Type `bool` |
| number | See **Literals** |
| string | Type `str` (hosted) |

### Literals (normative)

| Form | AST | Type rule |
|------|-----|-----------|
| Untyped int | `42` | Default **`i32`** if it fits; else type error (use typed literal) |
| Untyped float | `1.5` | Default **`f64`** |
| Typed int | `["lit", "i64", "42"]` | Exact width; value must fit |
| Typed float | `["lit", "f32", "1.5"]` | Exact width |
| Bool | `true`/`false` | `bool` |
| String | `"hi"` | `str` |
| Array | `["array_lit", ty?, expr...]` | If `ty` omitted, element type = unify(exprs); length = N → `[T; N]` |

### Places and binding

```json
["let", [[name, ty?, expr], ...], body]
["set!", name, expr]
["name", name]           // local / param read (or bare string name as expr — pick one)
```

**v0 choice (canonical):** variable reference is a bare JSON string **only when** it is a declared name; prefer explicit `["var", name]` to avoid ambiguity with `str` literals that look like names.

```json
["var", "x"]
```

Strings that are not under `["var", ...]` and not keywords are **string literals** (`str`).

### Control

```json
["seq", expr...]
["if", cond, then, else]          // else required; cond : bool
["loop", body]
["break", expr?]                  // value defaults to unit-less; v0 requires expr typed as loop result
["return", expr]
["call", callee, arg...]
["as", ty, expr]                  // explicit cast
```

### Match (normative in v0)

```json
["match", scrutinee, arm...]
arm := [pattern, expr]
pattern :=
  | ["ok", name] | ["err", name]           // for Result
  | ["var", name]                            // bind all
  | ["variant", enum_path, variant, bind...] // enum payload binds (0+)
  | "_"                                      // wildcard (JSON string)
```

`match` must be exhaustive for the scrutinee type (v0 checker requirement for `Result` and enums).

### Borrow / move (minimal hooks)

```json
["borrow", "shared" | "mut", place]
["move", place]
```

Full borrow checking is outside format completeness; nodes exist so later checkers share AST.

### Capability ops (hosted stub)

```json
["cap", op, arg...]
```

v0 allows `["cap", "print", expr]` for examples only when `main` is marked hosted. Exact cap typing is **v0.1+**; format reserves the tag.

## Arithmetic and compare (builtins)

Callee is a string builtin:

```json
["call", "+", a, b]
```

| Builtin | Operand types | Result |
|---------|---------------|--------|
| `+ - * / %` | same integer width, or same float width | same |
| `< <= > >=` | same ordered width | `bool` |
| `== !=` | same type (v0: primitives, `str`, pointers) | `bool` |
| `div_s` / `rem_s` | signed ints | `Result` optional later; v0 `/` on int is **unchecked** or typecheck-forbidden toward zero — **v0 rule: `/` and `%` on integers yield the same width; division by zero is `err` via recommended `["call", "checked_div", a, b] -> Result`** |

**v0 simplification:** plain `/` on integers **aborts** on division by zero; otherwise trunc toward zero. Prefer `checked_div` → `Result`. Integer `+ - *` **wrap**. See [PHASE1_DECISIONS.md](PHASE1_DECISIONS.md).

Mismatched widths require `["as", ty, expr]` first.

### Other v0 builtins

| Builtin | Meaning |
|---------|---------|
| `ok` / `err` | Construct `Result` values: `["call", "ok", v]`, `["call", "err", e]` |
| `checked_add` / `checked_sub` / `checked_mul` / `checked_div` | `i32 × i32 → Result[i32, str]`; `err` on overflow or div-by-zero |
| `aget` | `aget(arr, idx)` element load for `[T; N]` (OOB: interpreter abort in v0) |
| `aset` | `aset(arr, idx, v)` element store; v0 first arg must be `["var", name]`; returns `i32` `0` |
| `fset` | `fset(struct, field, v)` field store; v0 first arg must be `["var", name]`; field is a string name; returns `i32` `0` |

## Function pointer / vtable values (shape only)

```json
["fn_ptr", name]
["struct_lit", type_name, [field, expr]...]
```

Calling a fn pointer: `["call", expr_callee, args...]` where callee type is `["fn", ...]`.

### Variant construction (v0)

```json
["variant_lit", enum_name, variant_name]                 // unit
["variant_lit", enum_name, variant_name, payload...]     // 1+ payloads
```

Variant defs: `[name]` | `[name, ty]` | `[name, [ty...]]`.  
A second element that is an array whose head is a type constructor (`named`/`ref`/`array`/…) is a **single** compound `ty`; otherwise an array is a **tuple** of payload types.

Match: `["variant", enum_name, variant_name, bind...]` — one bind name per payload (none for unit). Exhaustive unless `_`.

### Field projection (v0)

```json
["field", place, field_name]
```

`place` must have type `["named", StructName]`. Returns the field type.  
v0: all-`Copy` field structs are `Copy`; field reads of `Copy` fields do not move the struct.  
Field store: builtin `fset` (see builtins table).

## Complete minimal example (sum 1..10)

```json
["mod", "sum",
  ["fn", "main", [], "i32",
    ["let", [["s", "i32", ["lit", "i32", "0"]],
             ["i", "i32", ["lit", "i32", "1"]]],
      ["loop",
        ["if",
          ["call", "<=", ["var", "i"], ["lit", "i32", "10"]],
          ["seq",
            ["set!", "s", ["call", "+", ["var", "s"], ["var", "i"]]],
            ["set!", "i", ["call", "+", ["var", "i"], ["lit", "i32", "1"]]]],
          ["break", ["var", "s"]]]]]]]
```

## Out of v0 (explicit)

- Generics / traits / `dyn`
- Full lifetime parameters
- Channels / tasks
- Modules imports multi-file
- GC collections
- Untyped `host` dynamic calls (use `cap`)

## Versioning

| Version | Meaning |
|---------|---------|
| air-format v0 (this draft) | Minimal typed AST for bootstrap programs |
| v0.1 | Caps typing, richer `main`, string in freestanding policy tooling |
| v1 | Stable after interpreter accepts the example suite |

Incompatible tag changes bump the format version and update [ROADMAP.md](ROADMAP.md).
