# AIR Vision

**AIR** (AI Intermediate Representation) is a **general-purpose systems language** whose canonical form is an explicit AST / execution IR.

It targets the same class of software as C, C++, and Rust — including **freestanding / kernel-level** code — while optimizing the **representation** for AI authors (agents) and machine clarity. It is not a dynamic scripting language and not a short-script DSL.

Human-oriented text syntax is secondary. Static types, explicit memory, and capability-gated effects are primary.

## Why AIR exists

C/C++/Rust already reach kernels and bare metal, but they are **text-first and human-first**. When the primary author is often an LLM, rich surface syntax and implicit effects become failure modes (cascading parse errors, hidden allocation, unclear aliasing).

AIR keeps **systems-level ambition** and changes the representation:

- **Canonical program = typed AST** (not text-first grammar).
- **Mnemonic view** for human inspection (projection, not source of truth).
- **Static types** with precise widths (`i32`, `i64`, `f64`, pointers, …) — not a single dynamic `num`.
- **No GC.** Memory follows **ownership + borrowing**, with **explicit allocators / arenas** for heap and freestanding patterns.
- **Dual lowering:** reference interpreter / VM for bring-up and agents; **native compilation** for production and kernel/freestanding targets.
- Token density remains a design constraint for AI generation.

## Who it is for

| Audience | Role |
|----------|------|
| Coding agents / LLMs | Emit and patch typed canonical AST; consume structured type/borrow diagnostics |
| Systems / application developers | Build userland and freestanding / kernel components in AIR |
| Toolchain authors | Typechecker, borrow/ownership checker, native backend, optional IR interpreter |
| Humans | Inspect mnemonic, types, and capability/effect audits |

## What AIR is not

- Not a dynamically typed language.
- Not GC-based (Java/Go/Python-style heaps are out of scope as the default runtime).
- Not a human-first sugar language (text is not canonical).
- Not related to CostGate or MCP token optimization.
- Not a multi-agent communication IR in the current scope (see [ROADMAP.md](ROADMAP.md)).

## Core propositions

1. **Systems-capable general-purpose** — userland *and* freestanding/kernel profiles.
2. **Statically typed** — precise integer/float/pointer types; typechecking before execution/codegen.
3. **Canonical form is AST** — tagged, typed nodes; dense and regular for models.
4. **Mnemonic is a view** — round-trip with AST required.
5. **Explicit memory** — ownership/move/borrow + explicit allocators/arenas; no hidden GC.
6. **Explicit effects** — capability-gated operations; freestanding cores omit hosted I/O.

## Success metrics (initial)

1. Typechecker rejects width/aliasing/ownership mistakes before run/codegen.
2. Freestanding subset links without a hosted runtime / GC.
3. Token density competitive with equivalent Rust/C for the same typed AST suite.
4. Agents spend time on semantic/type/borrow errors, not parse syntax.
5. Native backend can target at least one freestanding environment (e.g. `no_std`-equivalent profile).

## Non-goals (near term)

- Production-grade optimizing compiler parity with LLVM Rust on day one (path exists; quality is phased)
- Human sugar syntax as canonical form
- Mandatory transpile from C/Rust source text
- Communication IR (separate track)

## Related docs

- [DESIGN.md](DESIGN.md) — types, memory, execution, profiles
- [AIR_FORMAT.md](AIR_FORMAT.md) — minimal typed AST (air-format v0)
- [AI_NATIVE.md](AI_NATIVE.md) — memory/errors/process/shell/capabilities (AI-Native defaults)
- [EXAMPLES.md](EXAMPLES.md) — sketches (pending rewrite for static/systems model)
- [ROADMAP.md](ROADMAP.md) — phases
