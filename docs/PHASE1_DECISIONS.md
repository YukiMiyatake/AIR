# Phase 1 pre-decisions

Small normative choices locked before the bootstrap toolchain.  
Contract remains [SUBSET.md](SUBSET.md). Format remains [AIR_FORMAT.md](AIR_FORMAT.md).

## 1. Integer / float overflow

| Op | Default behavior (v0 interpreter) |
|----|-----------------------------------|
| `+ - *` on integers | **Wrap** (two’s complement), same width |
| `/ %` on integers | **Abort** on division by zero; otherwise trunc toward zero |
| `checked_add` / `checked_sub` / `checked_mul` / `checked_div` | Return `Result` (`err` on overflow or div-by-zero) |
| Float ops | IEEE-754; no traps on NaN/Inf in v0 |

Agents that need safety should emit `checked_*`, not rely on wrap.

## 2. `main` and process status

- Hosted `main() -> i32`: the returned `i32` **is** the process exit code.
- Interpreter CLI prints the value and exits with `status = value & 0xff` (shell-compatible).
- Freestanding entry is out of Phase 1.

### 2b. Program stdout vs CLI echo

- **`["cap", "print", expr]`** writes to **program stdout** (hosted only). This is the Phase 1 effect used by examples and smoke tests.
- The CLI’s echo of the `main` return value is **tooling output**, not a substitute for `cap.print`. Tests that care about effects must capture program stdout (or an interpreter capture hook), not only the exit code.
- stdin / stderr / fs / net are out of Phase 1 except as later stubs; see [ROADMAP.md](ROADMAP.md) hosted I/O priority table.

## 3. Diagnostics (machine-readable)

Toolchain emits **JSON Lines** (one object per diagnostic) on stderr when `--diag=json`, else human text on stderr.

```json
{
  "severity": "air",
  "code": "mem.use_after_move",
  "severity": "error",
  "message": "use of moved local `x`",
  "path": "examples/sum.air",
  "span": { "offset": 0, "length": 0 }
}
```

Stable code families:

- `type.*` — typechecker  
- `mem.*` — ownership ([OWNERSHIP.md](OWNERSHIP.md))  
- `parse.*` — format/JSON shape  
- `runtime.*` — interpreter abort (div0, OOB, …)

`span` may be zeros in Phase 1 if source map is not ready.

## 4. Execution strategy (Phase 1)

- **AST-walking interpreter** (no bytecode required for Phase 1).
- Bytecode remains a later optimization / native path concern ([DESIGN.md](DESIGN.md)).

## 5. Reference implementation language

| Stage | Host language | Location |
|-------|---------------|----------|
| Phase 1 bootstrap | **TypeScript / Node** | `tools/airc/` |
| Phase 1.5+ production | **Rust** | `crates/airc/` |

TS exists for fast bootstrap only. Production `airc` is Rust (speed, single binary, native codegen).  
**Docker is the supported way to develop** — see [TOOLING.md](TOOLING.md).  
Canonical program text is **`.air`**; `.air.json` is legacy parity ([ENCODING.md](ENCODING.md)).

## 6. CLI sketch

TypeScript (bootstrap):

```text
npm run airc -- check <file.air>
npm run airc -- run   <file.air>
```

Rust:

```text
cargo run -p airc -- version
cargo run -p airc -- check <file.air>
cargo run -p airc -- run   <file.air>
```

Docker:

```text
docker compose run --rm dev cargo run -p airc -- run examples/sum.air
docker compose run --rm dev npm run airc -- run examples/sum.air
```

`.air.json` remains accepted where noted; prefer `.air` for new work.

Exit codes: `0` ok; `1` check/runtime failure; `2` CLI usage error.  
For `run`, after a successful program, process status follows §2.
