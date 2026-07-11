# AIR Encodings

How the **same typed AST** (air-format) is stored, edited, reviewed, and packed.

The language’s source of truth is the **AST**, not a particular file syntax. Encodings are projections with different jobs.

## Layers

| Layer | Role | Human / git Diff | Token density | Status |
|-------|------|------------------|---------------|--------|
| **Canonical text: normalized S-expr** (`.air`) | Edit, PR, agent I/O, line-oriented review | Strong (1 node ≈ 1 line when formatted) | Good (no keyword quotes) | **Default** |
| **JSON array tree** (`.air.json`) | Legacy bootstrap / TS parity fixtures | Weaker (brackets/commas) | Poor | **Supported, not default** |
| **Binary tagged AST** (`.airb`) | Cache, distribution, fast load | Not for PR review | Best | Sketch (`airc pack`) |
| **Mnemonic view** | Human inspection | Strong | N/A (view only) | Documented in EXAMPLES |

```text
          ┌─────────────┐
   edit → │  .air S-expr │ ←── airc fmt (canonical pretty)
          └──────┬──────┘
                 │ parse
                 ▼
          ┌─────────────┐
          │  typed AST  │ ←── check / run / codegen
          └──────┬──────┘
        ┌────────┼────────┐
        ▼        ▼        ▼
   .air.json   .airb    mnemonic
   (legacy)   (pack)     (view)
```

## Design principles (agreed)

1. **Syntax tags are a closed set** — `fn`, `let`, `call`, `struct`, … may become compact binary opcodes. They version with air-format.
2. **User-defined names are open** — type names, field names, variants, function names live in a **symbol / path table**. Never assign a global binary tag to `Point` / `MyError`. Binary nodes reference **name IDs** (interned strings or module-local indices).
3. **Do not put binary in PRs by default** — review and agent patches use **normalized S-expr**. Binary is a derived artifact.
4. **Equality is structural** — compare canonical AST (or its hash), not raw bytes of an unnormalized encoding.  
   CLI: `airc hash <file>` (SHA-256 of compact JSON of the AST value) and `airc eq <a> <b>`.
5. **`airc fmt` is mandatory for text** — one pretty shape; no hand-formatting wars.

## Normalized S-expr (canonical text)

### Goals

- Keywords / tags are **bare atoms** — no `"fn"` quotes.
- Same AST as JSON `[tag, …]` trees.
- Formatter emits **line-oriented** layout so line Diff stays local.
- Round-trip: `parse(fmt(ast)) == ast` (structural).

### Sketch

```text
(mod sum
  (fn main
    ()
    i32
    (let
      ( (s i32 (lit i32 0))
        (i i32 (lit i32 1)) )
      (loop
        (if (<= i (lit i32 10))
          (seq
            (set! s (+ s i))
            (set! i (+ i (lit i32 1))))
          (break s))))))
```

### Lexical rules (v0.1)

| Token | Form |
|-------|------|
| Atom / keyword | `[A-Za-z_!][A-Za-z0-9_!./%<>=+-]*` (includes `set!`, `<=`, `+`) |
| Integer | `-?[0-9]+` (used for array lengths and `lit` digits) |
| String | `"…"` JSON-style escapes (for `str` literals / messages) |
| List | `( … )` — first element is usually a tag atom |

`lit` stores digits as strings in the AST (`["lit","i32","0"]`); the S-expr may write `(lit i32 0)` and the parser normalizes the digit atom to a string.

### Formatter rules (line Diff)

- **Block forms** (`mod`, `fn`, `let`, `seq`, `loop`, `if`, `match`, `array_lit`): leading atoms stay on the opening line; each nested form is on its own indented line.
- **`fn` signature** stays one head line: `(fn main () i32` then the body block.
- **`let` bindings**: one binding per line inside the bindings list.
- **Leaf / shallow forms** stay inline when short (≤ ~96 cols): `lit`, `var`, `call`, `set!`, `cap`, …
- Soft width and shallowness are implementation-defined but stable under `airc fmt`.
- Stable ordering: preserve AST child order (already canonical in air-format).

Example (`sum`):

```text
(mod sum
  (fn main () i32
    (let
      (
        (s i32 (lit i32 0))
        (i i32 (lit i32 1))
      )
      (loop
        (if
          (call <= (var i) (lit i32 10))
          (seq
            (set! s (call + (var s) (var i)))
            (set! i (call + (var i) (lit i32 1)))
          )
          (break (var s)))))))
```

## Binary tagged AST (`.airb` v1 sketch)

| Kind | Encoding |
|------|----------|
| Header | magic `AIRB` + `u8` version `1` |
| Symbol table | `u32` count + (`u16` len + UTF-8)* — **user names, field names, string lits** |
| Node tags (`fn`, `let`, `call`, …) | Closed `KNOWN_TAGS` index via opcode `0x40` (not in the symbol table as the tag atom) |
| Other arrays | opcode `0x05` + length + children |
| Numbers / bools | `i64` / bool opcodes |

**User-defined types do not expand the tag enum.**  
`["named", "Point"]` → tagged/`named` opcode path + **symbol id** for `"Point"`.

CLI (derived artifact; not for PRs by default):

```bash
airc pack examples/sum.air /tmp/sum.airb
airc unpack /tmp/sum.airb
```

PRs and agents still see S-expr; `pack` / `unpack` convert.

## JSON (legacy bootstrap)

Still accepted by `airc` for parity with early Phase 1 fixtures and TS oracle tests.  
**Do not author new programs or PR diffs in `.air.json`.** Prefer `.air` (and `airc fmt`). Both decode to the same AST.

## Tooling roadmap

| Step | Deliverable |
|------|-------------|
| Done | [ENCODING.md](ENCODING.md); Rust `fmt` + `.air` parse; `examples/sum.air` |
| Done | All Phase 1 examples as `.air` (+ `.air.json` parity for TS) |
| Done | `airc hash` / `airc eq` (structural AST identity) |
| Done | Line-oriented `fmt` (head-inline / body-block for Diff) |
| Done | TS `.air` S-expr parse (`parseModuleFile`) |
| Done | Binary `.airb` v1 sketch (`airc pack` / `unpack`) |
| Done | Docs/CLI: `.air` is the default; JSON is legacy |
| Later | Richer binary payloads; drop `.air.json` fixtures when TS CLI is retired |

## Non-goals

- Human sugar syntax as canonical source (see [VISION.md](VISION.md)).
- YAML / ad-hoc object JSON (`{"tag":"fn"}`) as interchange.
- Global numeric IDs for user type names across the ecosystem.
