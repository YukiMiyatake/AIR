# AIR Phase 1 reference CLI (`airc`) — TypeScript oracle

TypeScript / Node **oracle** for the Phase 1 suite.  
**Primary CLI is Rust** (`crates/airc`, `docker compose run --rm airc-rs`). See [docs/TOOLING.md](../../docs/TOOLING.md).

Canonical program text is **`.air`** (S-expr); `.air.json` is legacy parity — see [docs/ENCODING.md](../../docs/ENCODING.md).

```bash
npm install
npm run build
npm test
npm run airc -- check examples/sum.air   # optional; prefer Rust airc
```
