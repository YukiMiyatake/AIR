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

---

## Using these examples

Runnable JSON fixtures live under [`examples/`](../examples/):

| File | Expected `main` result |
|------|-------------------------|
| `sum.air.json` | `55` |
| `div.air.json` | `-1` (div by zero → err arm) |
| `arr.air.json` | `10` |
| `hello.air.json` | `0` (prints `hello`) |

```bash
docker compose run --rm dev cargo run -p airc -- run examples/arr.air.json
```

- Token benchmarks: minify JSON AST vs equivalent Rust/C for the same suite.
- Round-trip tests (when implemented) must accept these modules as air-format v0.
- Do not reintroduce dynamic `list` / untyped `num` / ad-hoc `host` tags.
