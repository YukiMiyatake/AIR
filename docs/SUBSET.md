# AIR Bootstrap Subset (Phase 1 cut)

Concrete **in / out** list so “systems language ambition” does not block a first working toolchain.  
Full product goals remain in [VISION.md](VISION.md); this file is the **near-term contract**.

## In scope for Phase 1 (must ship)

| Area | Included |
|------|----------|
| Format | [air-format v0](AIR_FORMAT.md) only |
| Types | integers/floats/`bool`/`str`(hosted)/`array`/`result`/`struct`/`enum` |
| Control | `seq` `let` `set!` `var` `if` `loop` `break` `return` `call` `match` `as` |
| Memory | [OWNERSHIP.md](OWNERSHIP.md) v0 moves + lexical borrows; **no heap libraries required** |
| Effects | optional `cap.print` for hosted hello |
| Tooling | parse → typecheck → ownership check → interpret examples in [EXAMPLES.md](EXAMPLES.md) |
| Lowering | interpreter only (native is Phase 2) |

## Explicitly out of Phase 1

| Area | Deferred to |
|------|-------------|
| Generics / traits / vtables as language features | Phase 3 ([ABSTRACTION.md](ABSTRACTION.md) layering still documented) |
| Full lifetime parameters | after Phase 1 |
| `Box`/`Vec` and general heap | Phase 3 stdlib + Alloc |
| Tasks / channels / threads | Phase 4 (rules sketched in [CONCURRENCY.md](CONCURRENCY.md)) |
| Freestanding kernel binary | Phase 2 |
| Multi-file modules | Phase 3 |
| Shell / proc / net caps | after Phase 1 (policy already in AI_NATIVE) |

## Definition of done (Phase 1)

1. All EXAMPLES modules typecheck + ownership-check under v0 rules.  
2. Interpreter returns expected `main` results for examples 1–3; example 4 prints in hosted mode.  
3. Ill-typed / use-after-move programs fail with stable diagnostic codes.  
4. No GC runtime linked.

## Naming note

**AIR** = AI Intermediate Representation as the **canonical program form** (typed AST). The project is still a **general-purpose systems language + toolchain** that uses that IR — not “IR-only / no language.”
