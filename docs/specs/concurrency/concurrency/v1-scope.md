# Concurrency And Behaviors v1 Scope

Status: `approved-pre-selfhost`

This scope freezes the current rewrite-era concurrency, task/thread, and behavior-stepping contract.

## Included Surface

- `async fn`
- `async fn main() -> Int | Unit`
- `weave`
- `split`
- `task_expr :: :: >>`
- `std.concurrent`
- `std.behaviors.step`
- `behavior[...] fn`
- `system[...] fn`

## Execution Contract

- `weave` is the task primitive.
- `split` is the thread primitive.
- Tasks and threads are distinct runtime concepts with distinct completion/join behavior.
- `thread_id` is a real runtime query, not a permanently constant placeholder.
- Main-thread-only behavior must remain explicit where required by the approved host/app substrate.

## Scheduler Contract

- The exact scheduler implementation remains a runtime/backend detail.
- The public contract still requires:
  - meaningful task/thread lifecycle
  - meaningful completion and join behavior
  - deterministic behavior/system phase stepping where the frozen contract requires it
  - explicit worker/main affinity rules where the approved substrate requires them
- Async and parallel chain execution must sit on the same real scheduler/worker substrate rather than on eager done-handle shims.

## Std Surface

- `std.concurrent` remains rewrite-owned first-party runtime surface.
- Channels, mutexes, atomics, tasks, and threads are part of that current surface.
- `std.behaviors.step` remains part of the first-party runtime language/substrate contract rather than showcase-only convenience.
