# AIR Phase 1 reference CLI (`airc`)

TypeScript / Node bootstrap toolchain.  
Canonical program text is **`.air`** (S-expr); `.air.json` is legacy parity — see [docs/ENCODING.md](../../docs/ENCODING.md).

Also see [docs/PHASE1_DECISIONS.md](../../docs/PHASE1_DECISIONS.md) and [docs/SUBSET.md](../../docs/SUBSET.md).

```bash
npm install
npm run build
npm run airc -- version
npm run airc -- check examples/sum.air
npm run airc -- run examples/sum.air
```

Prefer the Rust CLI under `crates/airc` for day-to-day use.
