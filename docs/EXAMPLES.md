# AIR Examples

Examples conform to **[air-format v0](AIR_FORMAT.md)**. Legacy dynamic/`num`/`list` sketches were removed.

## Example 1 — Sum 1..10 (`i32`)

Python equivalent:

```python
def main() -> int:
    s = 0
    i = 1
    while i <= 10:
        s += i
        i += 1
    return s
```

### Canonical AST

```json
["mod", "sum",
  ["fn", "main", [], "i32",
    ["let", [
        ["s", "i32", ["lit", "i32", "0"]],
        ["i", "i32", ["lit", "i32", "1"]]
      ],
      ["loop",
        ["if",
          ["call", "<=", ["var", "i"], ["lit", "i32", "10"]],
          ["seq",
            ["set!", "s", ["call", "+", ["var", "s"], ["var", "i"]]],
            ["set!", "i", ["call", "+", ["var", "i"], ["lit", "i32", "1"]]]],
          ["break", ["var", "s"]]]]]]]
```

### Mnemonic (informative)

```
fn main() -> i32
  const.i32 0
  store s
  const.i32 1
  store i
loop L0
  load i
  const.i32 10
  cmp.le.i32
  jump_if_false L1
  load s
  load i
  add.i32
  store s
  load i
  const.i32 1
  add.i32
  store i
  jump L0
L1:
  load s
  return
```

---

## Example 2 — `Result` + `match` (checked divide)

```json
["mod", "div",
  ["fn", "checked_div", [["a", "i32"], ["b", "i32"]], ["result", "i32", "str"],
    ["if",
      ["call", "==", ["var", "b"], ["lit", "i32", "0"]],
      ["call", "err", ["lit", "str", "div0"]],
      ["call", "ok", ["call", "/", ["var", "a"], ["var", "b"]]]]],
  ["fn", "main", [], "i32",
    ["match",
      ["call", "checked_div", ["lit", "i32", "10"], ["lit", "i32", "0"]],
      [["ok", "v"], ["var", "v"]],
      [["err", "e"], ["lit", "i32", "-1"]]]]]
```

Notes:

- `ok` / `err` constructors build `["result", T, E]` values (builtins).
- `match` arms are exhaustive for `Result`.

---

## Example 3 — Fixed array sum (no heap)

```json
["mod", "arr",
  ["fn", "main", [], "i32",
    ["let", [
        ["xs", ["array", "i32", 4],
          ["array_lit", "i32",
            ["lit", "i32", "1"],
            ["lit", "i32", "2"],
            ["lit", "i32", "3"],
            ["lit", "i32", "4"]]],
        ["s", "i32", ["lit", "i32", "0"]],
        ["i", "i32", ["lit", "i32", "0"]]
      ],
      ["loop",
        ["if",
          ["call", "<", ["var", "i"], ["lit", "i32", "4"]],
          ["seq",
            ["set!", "s",
              ["call", "+", ["var", "s"],
                ["call", "aget", ["var", "xs"], ["var", "i"]]]],
            ["set!", "i", ["call", "+", ["var", "i"], ["lit", "i32", "1"]]]],
          ["break", ["var", "s"]]]]]]]
```

`aget` is a v0 builtin: `aget(array[T;N], i32) -> T` (bounds: interpreter abort or later `Result`).

---

## Example 4 — Hosted print (`cap`)

```json
["mod", "hello",
  ["fn", "main", [], "i32",
    ["seq",
      ["cap", "print", ["lit", "str", "hello"]],
      ["lit", "i32", "0"]]]]
```

Requires hosted profile. Freestanding must not use `cap` print.  
**Phase 1 priority:** stdout via `cap.print` is required for example/smoke tests (see [ROADMAP.md](ROADMAP.md)); do not defer it with fs/net.

---

## Example 5 — Checked overflow (`checked_add`)

```json
["mod", "overflow",
  ["fn", "main", [], "i32",
    ["match",
      ["call", "checked_add", ["lit", "i32", "2147483647"], ["lit", "i32", "1"]],
      [["ok", "v"], ["var", "v"]],
      [["err", "e"], ["lit", "i32", "-1"]]]]]
```

Builtins (see [PHASE1_DECISIONS.md](PHASE1_DECISIONS.md)): `checked_add` / `checked_sub` / `checked_mul` / `checked_div` → `Result[i32, str]` (`err` on overflow or div-by-zero). Plain `+ - *` still wrap; plain `/` still aborts on div0.

---

## Example 6 — Array store (`aset`)

```json
["mod", "aset_ex",
  ["fn", "main", [], "i32",
    ["let", [
        ["xs", ["array", "i32", 3],
          ["array_lit", "i32",
            ["lit", "i32", "1"],
            ["lit", "i32", "2"],
            ["lit", "i32", "3"]]]
      ],
      ["seq",
        ["call", "aset", ["var", "xs"], ["lit", "i32", "1"], ["lit", "i32", "9"]],
        ["call", "aget", ["var", "xs"], ["lit", "i32", "1"]]]]]]
```

v0: first argument must be a local place `["var", name]`. Returns `i32` `0`. OOB aborts.

---

## Example 7 — Lexical borrow (`borrow` / `mem.borrow_conflict`)

Held shared borrow via `let` forbids `set!` on the place until the binding ends:

```json
["mod", "bad_borrow",
  ["fn", "main", [], "i32",
    ["let", [["x", "i32", ["lit", "i32", "1"]]],
      ["let", [["r", ["ref", "shared", "i32"], ["borrow", "shared", ["var", "x"]]]],
        ["seq",
          ["set!", "x", ["lit", "i32", "2"]],
          ["lit", "i32", "0"]]]]]]
```

After the inner `let` ends, `set!` / use is allowed again (`borrow_ok.air` → `7`).

---

## Example 8 — Struct literal + field (`struct` / `struct_lit` / `field`)

```json
["mod", "point",
  ["struct", "Point", [["x", "i32"], ["y", "i32"]]],
  ["fn", "main", [], "i32",
    ["let", [
        ["p", ["named", "Point"],
          ["struct_lit", "Point",
            ["x", ["lit", "i32", "3"]],
            ["y", ["lit", "i32", "4"]]]]
      ],
      ["call", "+",
        ["field", ["var", "p"], "x"],
        ["field", ["var", "p"], "y"]]]]]
```

Expected `main` → `7`.

---

## Example 9 — User enum (`enum` / `variant_lit` / `variant` match)

```json
["mod", "option",
  ["enum", "Opt", ["None"], ["Some", "i32"]],
  ["fn", "main", [], "i32",
    ["match",
      ["variant_lit", "Opt", "Some", ["lit", "i32", "42"]],
      [["variant", "Opt", "Some", "v"], ["var", "v"]],
      [["variant", "Opt", "None"], ["lit", "i32", "-1"]]]]]
```

Expected `main` → `42`. Non-exhaustive match fails check (`bad_enum_match.air` → `type.match`).

---

## Example 10 — Tuple enum payload

```json
["mod", "pair",
  ["enum", "PairE", ["Pair", ["i32", "i32"]]],
  ["fn", "main", [], "i32",
    ["match",
      ["variant_lit", "PairE", "Pair", ["lit", "i32", "3"], ["lit", "i32", "4"]],
      [["variant", "PairE", "Pair", "a", "b"],
        ["call", "+", ["var", "a"], ["var", "b"]]]]]]
```

Expected `main` → `7`.

---

## Example 11 — Field store (`fset`)

```json
["mod", "fset_ex",
  ["struct", "Point", [["x", "i32"], ["y", "i32"]]],
  ["fn", "main", [], "i32",
    ["let", [
        ["p", ["named", "Point"],
          ["struct_lit", "Point",
            ["x", ["lit", "i32", "3"]],
            ["y", ["lit", "i32", "4"]]]]
      ],
      ["seq",
        ["call", "fset", ["var", "p"], "x", ["lit", "i32", "10"]],
        ["field", ["var", "p"], "x"]]]]]
```

v0: first argument must be `["var", name]`. Returns `i32` `0` from `fset`; example reads `x` → `10`.

---

## Using these examples

Primary fixtures are **S-expr** (`.air`). JSON (`.air.json`) is **legacy parity** for the TS bootstrap — do not author new examples in JSON — see [ENCODING.md](ENCODING.md).

| File | Expected `main` result |
|------|-------------------------|
| `sum.air` | `55` |
| `div.air` | `-1` (div by zero → err arm) |
| `arr.air` | `10` |
| `hello.air` | `0` (prints `hello`) |
| `bad_move.air` | **check fails** (`mem.use_after_move`) |
| `overflow.air` | `-1` (`checked_add` overflow → err arm) |
| `aset.air` | `9` |
| `bad_borrow.air` | **check fails** (`mem.borrow_conflict`) |
| `borrow_ok.air` | `7` |
| `point.air` | `7` (`struct` + `field`) |
| `option.air` | `42` (`enum` + `variant` match) |
| `pair.air` | `7` (tuple enum payload) |
| `fset.air` | `10` (`fset` field store) |
| `bad_enum_match.air` | **check fails** (`type.match` non-exhaustive) |

```bash
docker compose run --rm dev cargo run -p airc -- run examples/arr.air
docker compose run --rm dev cargo run -p airc -- fmt examples/sum.air
```

- Token benchmarks: minify JSON AST vs S-expr vs equivalent Rust/C for the same suite.
- Round-trip: `.air` AST must match sibling `.air.json` (tested in Rust) until JSON fixtures are retired.
- Do not reintroduce dynamic `list` / untyped `num` / ad-hoc `host` tags.
