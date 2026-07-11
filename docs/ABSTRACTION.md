# AIR Abstraction Layers

How **capabilities**, **traits/interfaces**, and **explicit vtables** relate. They are not three competing OOP systems — they answer different questions.

## One-line each

| Layer | Question it answers |
|-------|---------------------|
| **Capability (`cap`)** | *May* this program perform this effect / hold this authority? |
| **Trait / interface** | *What operations* does this type support (static contract)? |
| **Explicit vtable / ops struct** | *Which concrete function pointers* should be used at runtime? |

## Layering diagram

```text
┌─────────────────────────────────────────┐
│ Capability (authority / policy)         │  cap.fs, cap.net, cap.print, …
│  — values passed into main / boundaries │
└─────────────────┬───────────────────────┘
                  │ grants the right to use
                  ▼
┌─────────────────────────────────────────┐
│ Trait / interface (static contract)     │  trait Fs { read; write; … }
│  — compile-time obligations             │
└─────────────────┬───────────────────────┘
                  │ may be implemented by
                  ▼
┌─────────────────────────────────────────┐
│ Impl OR explicit vtable (dispatch)      │  static monomorphized impl
│  — who runs when you call               │  OR FileOps { read, write, ctx }
└─────────────────────────────────────────┘
```

## When to use which

| Situation | Use |
|-----------|-----|
| Deny shell / net by default | **Capability** only |
| Library API: “anything readable” | **Trait** (+ DI of the impl) |
| Test double / driver swap at runtime | **Explicit vtable** (or trait object later) |
| Kernel device ops table | **Explicit vtable** (often without hosted caps) |
| Zero-cost generic algorithm | **Trait + monomorphization** (no vtable) |
| Freestanding with no OS | Traits/vtables for structure; **no hosted caps** |

## Rules of thumb (normative)

1. **Capabilities never replace traits.** A `cap.fs` without an `Fs` API still needs typed operations.
2. **Traits never grant authority.** Implementing `Fs` does not let you touch the disk without a capability at the boundary.
3. **Vtables are data.** Prefer passing `FileOps` (or a trait impl) explicitly — no global prototype mutation.
4. **Bootstrap (air-format v0):** may use `cap` stubs and concrete fns only; traits/vtables arrive in Phase 3 but the *layering story* is fixed now.
5. **DI:** inject caps at the edge; inject traits/vtables inward for testability.

## Anti-patterns

| Anti-pattern | Why rejected |
|--------------|--------------|
| Ambient `open()` with no cap | Hidden authority |
| JS prototype patch for mocks | Hidden dispatch; not AI-auditable |
| “God” capability that implies all traits | Too coarse; breaks least privilege |
| Trait method that secretly shell-executes | Effect not in signature / caps |

## Worked sketch (hosted file read)

```text
main(caps: { fs: CapFs })
  let api: FileOps = real_file_ops(caps.fs)   // or mock_file_ops() in tests
  run(api)

fn run(api: FileOps)
  api.read(...)
```

- **Cap** proves `main` was allowed to build real ops.
- **Vtable** (`FileOps`) is what `run` depends on (mockable).
- A future **trait** `Fs` can be the static face of the same ops.

## See also

- [AI_NATIVE.md](AI_NATIVE.md) — cap and DI policy
- [DESIGN.md](DESIGN.md) § Abstraction, DI, and mocks
- [AIR_FORMAT.md](AIR_FORMAT.md) — `cap` tag in v0
