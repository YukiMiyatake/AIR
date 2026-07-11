# AIR Vision

**AIR** (AI Intermediate Representation) is a **general-purpose language** whose canonical form is an execution IR, plus a VM that runs it.

It is **not** a short-script DSL or a thin agent glue language. Agents and humans use the same language for real programs: libraries, long-running services, data processing, and tools. The differentiator is **AI-first representation**, not a reduced problem domain.

Human-oriented text syntax is secondary. Machine clarity, compact tokens, and explicit effects remain primary.

## Why AIR exists

Mainstream general-purpose languages (Python, JavaScript, Go, …) are text-first and human-first: rich syntax, implicit effects, and parsers that turn small mistakes into cascading failures. That is a poor fit when the primary author is often an LLM, even when the **programs themselves** are large and general-purpose.

AIR keeps the ambition of a general-purpose language, but changes the representation:

- The **canonical program is an explicit AST**, not a text-first grammar.
- A **mnemonic view** exists for humans to inspect and debug; it is a projection, not the source of truth.
- A **VM** runs AIR; side effects go through an explicit **host / runtime capability** boundary.
- Token cost per unit of meaning is a first-class design constraint.
- Language power (closures, modules, concurrency, libraries) is in scope — delivered in phases, not denied by product identity.

## Who it is for

| Audience | Role |
|----------|------|
| Coding agents / LLMs | Emit and patch canonical AST for full programs; consume structured diagnostics |
| Application / systems developers | Build real software on AIR; use mnemonic and tooling for inspection |
| Host / embedders | Embed the VM; expose permissioned capabilities; audit effects |
| Humans | Read mnemonic, traces, and docs; may author via tools that emit AST |

## What AIR is not

- Not a human-first sugar language (text syntax is not the source of truth).
- Not limited to “few-line agent scripts.”
- Not related to CostGate or MCP token optimization — AIR is an independent project.
- Not a communication / multi-agent protocol IR in the current scope (see [ROADMAP.md](ROADMAP.md)).

## Core propositions

1. **General-purpose** — express application logic, libraries, concurrency, and I/O-capable programs, not only glue.
2. **Canonical form is AST** — tagged nodes (S-expression / compact array form). Verbose key-heavy JSON is not canonical.
3. **Mnemonic is a view** — round-trip with AST is required.
4. **Tokens carry meaning densely** — short, regular encodings that models generate reliably.
5. **Effects are explicit** — capability-gated host/runtime operations; core evaluation rules stay auditable.

## Success metrics (initial)

1. **Token density** — for a fixed suite spanning non-trivial programs (not only toy loops), AIR canonical form uses fewer tokenizer tokens than equivalent Python (or similar) source.
2. **Syntax vs meaning** — failure modes shift from parse/syntax errors toward semantic / type-ish / capability / logic errors.
3. **Human inspectability** — developers can follow failures via mnemonic + capability audit log.
4. **Expressiveness path** — roadmap features (modules, closures, concurrency) land without abandoning AST-canonical / AI-first constraints.
5. **Capability accountability** — effectful operations are loggable and denyable by policy.

## Non-goals (near term only)

These are **deferred engineering**, not permanent exclusions from a general-purpose language:

- Production-grade JIT and GC tuning (later performance work)
- Human-oriented surface syntax as the canonical form
- Mandatory transpile-to / from existing languages
- Full static typing product (optional checks may come earlier)
- Inter-agent communication IR (separate track; see [ROADMAP.md](ROADMAP.md))

MVP may ship a **subset** of the language. The subset is a bootstrap, not the definition of AIR.

## Related docs

- [DESIGN.md](DESIGN.md) — execution model and design decisions
- [EXAMPLES.md](EXAMPLES.md) — AST / mnemonic / bytecode sketches
- [ROADMAP.md](ROADMAP.md) — phases toward a full general-purpose language
