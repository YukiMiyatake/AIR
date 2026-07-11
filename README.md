# AIR

**AI Intermediate Representation** — an AI-first language and VM.

AIR is designed for agents that write and execute code: compact token usage, AST-native structure, and a human mnemonic layer for inspection. Human readability is secondary; machine clarity is primary.

## Status

Early design. Language, bytecode, and VM are under active exploration.

## Goals

- Optimize for AI generation and understanding, not human ergonomics
- Minimize tokens per unit of meaning
- Treat programs as explicit AST / IR, not text-first syntax
- Provide a mnemonic view for humans (assembly-like), separate from the canonical form
- Ship a small VM that runs AIR directly

## License

TBD
