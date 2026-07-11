# AIR Encodings

How the **same typed AST** (air-format) is stored, edited, reviewed, and packed.

The language’s source of truth is the **AST**, not a particular file syntax. Encodings are projections with different jobs.

## Layers

| Layer | Role | Human / git Diff | Token density | Status |
|-------|------|------------------|---------------|--------|
| **Canonical text: normalized S-expr** (`.air`) | Edit, PR, agent I/O, line-oriented review | Strong (1 node ≈ 1 line when formatted) | Good (no keyword quotes) | **v0.1 target** (shipping now for `sum`) |
| **JSON array tree** (`.air.json`) | Phase 1 bootstrap wire format | Weaker (brackets/commas) | Poor | **v0 bootstrap** (still supported) |
| **Binary tagged AST** (`.airb` later) | Cache, distribution, fast load | Not for PR review | Best | Later (`airc pack`) |
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

- Opening tag on its line with structure; nested lists indented by 2 spaces.
- Prefer **one primary subform per line** for `seq`, `let` bindings, `fn` items.
- Short atoms may stay inline when the whole list fits a soft width (implementation-defined; default 96).
- Stable ordering: preserve AST child order (already canonical in air-format).

## Binary tagged AST (later)

| Kind | Encoding |
|------|----------|
| Node tags (`Fn`, `Let`, `Call`, …) | Closed `u8`/`u16` enum per air-format version |
| User / path / field names | Interned symbol table; nodes store IDs |
| Literals | Typed payloads (i32, bytes, …) |

**User-defined types do not expand the tag enum.**  
`["named", "Point"]` → `NamedTy` opcode + `sym_id("Point")`.

PRs and agents still see S-expr; `airc pack` / `unpack` convert.

## JSON (bootstrap)

Retained for Phase 1 examples and TS tooling. Prefer `.air` for new fixtures once `fmt` / parse are stable. Both decode to the same AST.

## Tooling roadmap

| Step | Deliverable |
|------|-------------|
| Done | [ENCODING.md](ENCODING.md); Rust `fmt` + `.air` parse; `examples/sum.air` |
| Done | All Phase 1 examples as `.air` (+ `.air.json` parity for TS) |
| Next | AST hash / structural eq CLI; improve line-oriented `fmt` |
| Later | Binary `.airb` pack; deprecate JSON as default in docs |

## Non-goals

- Human sugar syntax as canonical source (see [VISION.md](VISION.md)).
- YAML / ad-hoc object JSON (`{"tag":"fn"}`) as interchange.
- Global numeric IDs for user type names across the ecosystem.
