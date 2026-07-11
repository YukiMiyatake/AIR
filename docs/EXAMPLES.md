# AIR Examples

Same programs shown as **canonical AST**, **mnemonic view**, and a **bytecode sketch**. These are informative sketches until the bootstrap compiler exists; they must stay consistent with [DESIGN.md](DESIGN.md).

These examples are small for clarity. They do **not** imply AIR is limited to short scripts — see [VISION.md](VISION.md) (general-purpose language).

## Example 1 — Sum 1..n

Python equivalent (for token comparison later):

```python
def main():
    n = 10
    s = 0
    i = 1
    while i <= n:
        s += i
        i += 1
    print(s)
```

### Canonical AST

```json
[
  "mod",
  [
    "fn",
    "main",
    [],
    [
      "let",
      [
        ["n", 10],
        ["s", 0],
        ["i", 1]
      ],
      [
        "seq",
        [
          "loop",
          [
            "if",
            ["call", "<=", "i", "n"],
            [
              "seq",
              ["let", [["s", ["call", "+", "s", "i"]]], ["seq"]],
              ["let", [["i", ["call", "+", "i", 1]]], ["seq"]]
            ],
            ["break", "s"]
          ]
        ],
        ["host", "print", "s"]
      ]
    ]
  ]
]
```

Note: the `let`-rebind pattern above is illustrative. A compiler may lower mutating locals to slot stores without nested `let`. Prefer the mnemonic / bytecode view for mutation clarity.

### Cleaner AST (slot-mutation friendly)

Using explicit assign form allowed as sugar over slots (documented here for examples; compiler-internal):

```json
[
  "mod",
  [
    "fn",
    "main",
    [],
    [
      "seq",
      ["set!", "n", 10],
      ["set!", "s", 0],
      ["set!", "i", 1],
      [
        "loop",
        [
          "if",
          ["call", "<=", "i", "n"],
          [
            "seq",
            ["set!", "s", ["call", "+", "s", "i"]],
            ["set!", "i", ["call", "+", "i", 1]]
          ],
          ["break", "s"]
        ]
      ],
      ["host", "print", "s"]
    ]
  ]
]
```

`set!` is an AST convenience for local store; it is not a host effect.

### Mnemonic

```
fn main
  const 10
  store n
  const 0
  store s
  const 1
  store i
loop L0
  load i
  load n
  cmp_le
  jump_if_false L1
  load s
  load i
  add
  store s
  load i
  const 1
  add
  store i
  jump L0
L1:
  load s
  host print 1
  return
```

### Bytecode sketch

```
CONST 10
STORE 0          ; n
CONST 0
STORE 1          ; s
CONST 1
STORE 2          ; i
; L0:
LOAD 2
LOAD 0
CMP_LE
JUMP_IF_NOT L1
LOAD 1
LOAD 2
ADD
STORE 1
LOAD 2
CONST 1
ADD
STORE 2
JUMP L0
; L1:
LOAD 1
HOST print 1
RET
```

---

## Example 2 — Result instead of exceptions

Divide with an `err` on zero divisor; print ok value or error code.

### Canonical AST

```json
[
  "mod",
  [
    "fn",
    "div",
    ["a", "b"],
    [
      "if",
      ["call", "==", "b", 0],
      ["err", "div0", "division by zero"],
      ["ok", ["call", "/", "a", "b"]]
    ]
  ],
  [
    "fn",
    "main",
    [],
    [
      "let",
      [["r", ["call", "div", 10, 0]]],
      [
        "if",
        ["call", "is_ok", "r"],
        ["host", "print", ["call", "unwrap", "r"]],
        ["host", "print", ["call", "err_code", "r"]]
      ]
    ]
  ]
]
```

### Mnemonic (main only)

```
fn main
  const 10
  const 0
  call div 2
  store r
  load r
  is_ok
  jump_if_false Lerr
  load r
  unwrap
  host print 1
  jump Lend
Lerr:
  load r
  err_code
  host print 1
Lend:
  return
```

---

## Example 3 — List fold (no closures)

Sum a list with an index loop (closures are out of scope for MVP).

### Canonical AST

```json
[
  "mod",
  [
    "fn",
    "main",
    [],
    [
      "seq",
      ["set!", "xs", ["list", 1, 2, 3, 4]],
      ["set!", "s", 0],
      ["set!", "i", 0],
      ["set!", "n", ["call", "len", "xs"]],
      [
        "loop",
        [
          "if",
          ["call", "<", "i", "n"],
          [
            "seq",
            [
              "set!",
              "s",
              ["call", "+", "s", ["call", "get", "xs", "i"]]
            ],
            ["set!", "i", ["call", "+", "i", 1]]
          ],
          ["break", "s"]
        ]
      ],
      ["host", "print", "s"]
    ]
  ]
]
```

### Mnemonic

```
fn main
  list 1 2 3 4
  store xs
  const 0
  store s
  const 0
  store i
  load xs
  len
  store n
loop L0
  load i
  load n
  cmp_lt
  jump_if_false L1
  load s
  load xs
  load i
  get
  add
  store s
  load i
  const 1
  add
  store i
  jump L0
L1:
  load s
  host print 1
  return
```

---

## Using these examples

- Token benchmarks should compare **canonical AST JSON** (minified) vs Python source for the same suite.
- Round-trip tests (when implemented) must accept the AST forms above and regenerate equivalent mnemonic.
- Host `print` arguments are values; formatting is host-defined.
