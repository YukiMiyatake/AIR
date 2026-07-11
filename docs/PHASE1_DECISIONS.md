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

## 3. Diagnostics (machine-readable)

Toolchain emits **JSON Lines** (one object per diagnostic) on stderr when `--diag=json`, else human text on stderr.

```json
{
  "severity": "air",
  "code": "mem.use_after_move",
  "severity": "error",
  "message": "use of moved local `x`",
  "path": "examples/sum.air.json",
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

## 5. Reference implementation language (Phase 1)

- Phase 1 reference toolchain is **TypeScript on Node.js** (JSON AST is native; fast to bootstrap).
- Layout: `tools/airc/` (CLI) + packages under `tools/` as needed.
- A Rust (or other) native compiler remains valid for Phase 2+; this does not freeze the forever implementation language.

## 6. CLI sketch

```text
npx airc check <file.air.json>
npx airc run   <file.air.json>
npx airc run   <file.air.json> --diag=json
```

Exit codes: `0` ok; `1` check/runtime failure; `2` CLI usage error.  
For `run`, after a successful program, process status follows §2.
