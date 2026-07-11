# AIR Vision

**AIR** (AI Intermediate Representation) is an execution IR and small VM built for agents that generate, edit, and run code.

Human readability is secondary. Machine clarity, compact tokens, and sandboxable execution are primary.

## Why AIR exists

Agents today emit Python, JavaScript, or shell. Those languages optimize for humans: rich syntax, implicit I/O, large standard libraries, and parsers that punish small mistakes with cascading syntax errors.

AIR inverts that tradeoff:

- The **canonical program is an explicit AST**, not a text-first grammar.
- A **mnemonic view** exists for humans to inspect and debug; it is a projection, not the source of truth.
- A **small deterministic VM** runs AIR with side effects only through an explicit host API.
- Token cost per unit of meaning is a first-class design constraint.

## Who it is for

| Audience | Role |
|----------|------|
| Coding agents / LLMs | Emit and patch canonical AST; consume VM errors as structured feedback |
| Host runtimes | Embed the VM; expose a permissioned host API; audit host calls |
| Humans | Inspect mnemonic dumps and execution traces; rarely author by hand |

## What AIR is not

- Not a human-first programming language with sugar and soft syntax.
- Not a replacement for general-purpose application languages.
- Not related to CostGate or MCP token optimization — AIR is an independent project.
- Not a communication / multi-agent protocol IR in the current scope (see [ROADMAP.md](ROADMAP.md)).

## Core propositions

1. **Canonical form is AST** — tagged nodes (S-expression / compact array form). Verbose key-heavy JSON is not canonical.
2. **Mnemonic is a view** — one instruction per line, slots explicit; round-trip with AST is required.
3. **Tokens carry meaning densely** — short, regular encodings that models generate reliably.
4. **Effects are explicit** — only `host.op(...)` performs I/O; the VM itself is deterministic given host replies.

## Success metrics (initial)

Measurable once an example suite and MVP VM exist:

1. **Token density** — for a fixed example suite, AIR canonical form uses fewer tokenizer tokens than equivalent Python source.
2. **Syntax vs meaning** — agent failure modes shift from parse/syntax errors toward semantic / host / logic errors (parser is trivial).
3. **Human inspectability** — a developer can follow a failing run using mnemonic + host audit log alone.
4. **Sandbox accountability** — every host call is loggable and denyable by policy.

## Non-goals (v0 / early phases)

- JIT, advanced GC tuning, or high-performance numerics
- Human-oriented syntactic sugar or IDE-first ergonomics
- Mandatory transpile-to / from existing languages
- Static typing (deferred)
- Closures, multi-file modules (deferred; see [DESIGN.md](DESIGN.md) and [ROADMAP.md](ROADMAP.md))
- Inter-agent communication IR (explicitly later)

## Related docs

- [DESIGN.md](DESIGN.md) — execution model and fixed design decisions
- [EXAMPLES.md](EXAMPLES.md) — AST / mnemonic / bytecode sketches
- [ROADMAP.md](ROADMAP.md) — phases
