# AIR Concurrency (hosted sketch)

Hosted concurrency must respect **no GC** and **explicit allocation**. This document fixes the gap between “tasks + channels” and the memory model.

Freestanding/kernel concurrency (ISRs, atomics) is separate and does not require this runtime.

## Alloc owns channel buffers

| Object | Storage |
|--------|---------|
| Task stack / frame | Runtime-managed (fixed or growable under a **runtime Alloc** configured at start) |
| Channel queue | Buffer memory from an **`Alloc` passed when the channel is created** |
| Captures in spawned closures | Same rules as [OWNERSHIP.md](OWNERSHIP.md); heap captures need Alloc |

```text
chan_new[T](alloc: Alloc, capacity: usize) -> Chan[T]
chan_send(ch, value: T) -> Result[(), SendError]
chan_recv(ch) -> Result[T, RecvError]
spawn(alloc: Alloc, f: fn()) -> Task
```

- Closing/dropping a channel frees its buffer via the same Alloc.  
- A channel must not outlive its Alloc/arena.  
- **No** implicit global heap for queues.

## Scheduling

- Phase 4 target: M:N lightweight tasks in hosted runtime.  
- Deterministic tests: optional round-robin scheduler with a seed (AI-Native replay).  
- Shared mutability across tasks: mutex/atomics only; otherwise ownership prevents sharing.

## Phase alignment

- **Phase 1:** no tasks/channels in the bootstrap subset ([SUBSET.md](SUBSET.md)).  
- **Phase 4:** implement this sketch.  
- Until then, examples must not use concurrency tags.

## See also

- [AI_NATIVE.md](AI_NATIVE.md) § Concurrency  
- [DESIGN.md](DESIGN.md) § Concurrency  
- [OWNERSHIP.md](OWNERSHIP.md)
