# AIR Ownership (v0 operational rules)

Operational rules for **ownership, `set!`, borrows, and moves** in the bootstrap checker.  
Goal: enough precision for agents and implementers **without** full lifetime polymorphism (that is later).

Companion: [DESIGN.md](DESIGN.md), [AIR_FORMAT.md](AIR_FORMAT.md).

## Places and values

- A **local** is a named slot created by `fn` params or `let`.
- A **place** is a local, a field projection, or an array element place (`aget`/`aset` targets).
- Values are either **`Copy`** (primitives, `bool`, shared refs themselves are `Copy` in v0) or **affine** (moved on use unless borrowed).

v0 `Copy` set: all integer/float/`bool`/`str` (hosted), shared `&T`, function pointers.  
Structs are `Copy` iff all fields are `Copy` and marked/derived `Copy` (v0: only all-`Copy` field structs auto-`Copy`).

## Moves

- Reading an affine local with `["var", x]` **moves** `x` unless the use is under `["borrow", ...]`.
- After move, `x` is **uninitialized** until `set!` or a new `let` binding in an inner scope (v0: no re-let same name in same block; use `set!` to reinitialize).
- `["move", place]` forces a move (same rules).
- Passing an affine value to `call` moves unless the callee type takes `["ref", ...]`.

## `set!`

```json
["set!", name, expr]
```

Rules:

1. `name` must refer to an existing local in the current function.
2. `expr` type must equal the local’s declared type.
3. `set!` **writes** the slot. It is allowed when:
   - the local is uninitialized (after move), or
   - the local is initialized and **not currently borrowed**.
4. `set!` does not allocate; it overwrites the slot. Drop glue for the previous affine value runs before the write (v0 interpreter: conceptual drop; native: later).

`set!` on a borrowed local is a **check error** (`mem.borrow_conflict`).

## Borrows (lexical / region — v0)

```json
["borrow", "shared", place]
["borrow", "mut", place]
```

### Lifetime = enclosing expression / statement region

v0 does **not** parse `'a` lifetime parameters. A borrow lives until the end of the **immediate parent** `seq` element, `let` initializer, `call` argument evaluation, or `match` arm expression that contains it — whichever applies (the smallest enclosing expr that is not the borrow node itself).

Practical rule for the checker:

1. While a **`mut` borrow** of place `p` is live, no other borrow of `p` and no `set!`/`move` of `p`.
2. While one or more **shared borrows** of `p` are live, no `mut` borrow, no `set!`, no `move` of `p`.
3. Returning a borrow from a function is **v0-forbidden** (forces later lifetimes). Body-local borrows only.
4. Storing a borrow into a struct field that outlives the region is **v0-forbidden**.

This is intentionally stricter than Rust — easier for agents; expand later.

## Arrays

- `[T; N]` in a local is one place; `aset` requires no active borrow of that local (v0: treat whole array as the place).
- A struct local is one place; `fset` updates a field through `["var", name]` (v0: treat whole struct as the place for borrow).
- `aget` by value on `Copy` elements copies; on affine elements moves out (v0: only `Copy` elements in examples).

## Function calls and ownership

Callee parameter types decide:

| Param type | Argument |
|------------|----------|
| `T` (owned) | move/copy into callee |
| `["ref", "shared", T]` | shared borrow for the call region |
| `["ref", "mut", T]` | mut borrow for the call region |

## Drops

- At end of scope (`let` body / function), initialized affine locals are dropped in reverse order.
- `break` / `return` drop remaining locals after moving out the result.

## Error codes (stable)

| Code | Meaning |
|------|---------|
| `mem.use_after_move` | read/borrow of uninitialized local |
| `mem.borrow_conflict` | set!/move/borrow violates aliasing |
| `mem.borrow_escape` | borrow would outlive v0 region (return/store) |
| `mem.type_mismatch` | set!/move type mismatch |

## Explicit non-goals (v0)

- Lifetime parameters on `fn` / refs  
- Self-referential structs  
- Interior mutability types  
- Concurrent borrows across tasks (see later concurrency doc)

## Relationship to Alloc

Ownership rules apply to values. **Heap** allocation still requires an explicit `Alloc`/arena argument when libraries introduce `Box`/`Vec` (not in air-format v0 core examples). An arena end **drops** all values in that arena; dangling borrows into an ended arena are `mem.borrow_escape` / use-after-arena (later code).
