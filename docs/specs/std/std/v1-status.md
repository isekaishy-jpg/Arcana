# Arcana Standard Library v1 Status

Status: `approved-pre-selfhost`

This ledger tracks bootstrap-readiness for the rewrite-owned `std` surface.

Rules:
- Every `std` surface change must update this ledger or `docs/specs/std/std/v1-scope.md` in the same patch.
- Transitional carried modules must include an update note that states what still needs to change before they can be treated as rewrite-owned.
- This ledger may classify or track `std`; it may not expand `std` surface by itself.
- Family entries cover their narrow wrapper/root modules unless noted otherwise. For example, collection-family status covers `std.collections.*` plus the thin `std.list` / `std.array` compatibility wrapper modules.
- The current ledger is intended to support a pre-selfhost freeze of std shape: unless Milestone 6/runtime work or owned-grimoire development proves a real blocker, std should now move by implementation and verification rather than by repeated architectural redesign.

## Bootstrap-Required

id: STD-ARGS
classification: removed-from-std
why: public host-core argument access now belongs to `arcana_process.args`
consumers: owned host-core tools and future owned compiler/tooling entrypoints through `arcana_process`
current_source: removed-from-std
still_needs_rebuild: none in std; keep the active surface owned by `arcana_process`
update_note: the old public std host-core lane is retired; low-level argument access remains public, but it is no longer a std responsibility
promotion_condition: only revisit if a future explicit scope moves host-core back into std

id: STD-ENV
classification: removed-from-std
why: public environment access now belongs to `arcana_process.env`
consumers: host-tool and backend/runtime bootstrap lanes through `arcana_process`
current_source: removed-from-std
still_needs_rebuild: none in std; keep the active surface low-level in `arcana_process.env`
update_note: std no longer owns public env access; higher-level config loading still stays out of the low-level host-core lane
promotion_condition: only revisit if a future explicit scope moves host-core back into std

id: STD-PATH
classification: removed-from-std
why: public path handling now belongs to `arcana_process.path`
consumers: package/build runtime, owned host-core tools, future owned compiler/tooling flows through `arcana_process`
current_source: removed-from-std
still_needs_rebuild: none in std; keep the active path surface aligned with approved host-core scope in `arcana_process.path`
update_note: the public host-core path surface moved out of std; the approved helper set remains the same, but the owner is now `arcana_process`
promotion_condition: only revisit if a future explicit scope moves host-core back into std

id: STD-FS
classification: removed-from-std
why: public filesystem access now belongs to `arcana_process.fs`
consumers: package/build runtime, owned host-core tools, future owned compiler/tooling flows through `arcana_process`
current_source: removed-from-std
still_needs_rebuild: none in std; keep filesystem APIs substrate-level under `arcana_process.fs`
update_note: the public fs surface moved out of std; `FileStream` and the approved explicit helper set remain active under `arcana_process.fs`
promotion_condition: only revisit if a future explicit scope moves host-core back into std

id: STD-PROCESS
classification: removed-from-std
why: public process execution now belongs to `arcana_process.process`
consumers: owned host-core tools through `arcana_process`
current_source: removed-from-std
still_needs_rebuild: none in std; keep the active process surface narrow in `arcana_process.process`
update_note: std no longer owns public process execution; the active low-level status/capture surface remains in `arcana_process.process`
promotion_condition: only revisit if a future explicit scope moves host-core back into std

id: STD-IO
classification: removed-from-std
why: public host-core IO now belongs to `arcana_process.io`
consumers: compiler grimoires, host-tool examples, showcase examples through `arcana_process`
current_source: removed-from-std
still_needs_rebuild: none in std; keep the active IO surface low-level in `arcana_process.io`
update_note: the public std IO lane is retired; stdout/stderr output and line input remain available through `arcana_process.io`, not std
promotion_condition: only revisit if a future explicit scope moves host-core back into std

id: STD-CONFIG
classification: bootstrap-required
why: deterministic section/key config parsing substrate for Arcana-side manifest/config readers without coupling std to the Rust-side package parser
consumers: `std.manifest`, future Arcana-owned tooling/config readers
current_source: rewrite-owned
still_needs_rebuild: keep the parser generic enough for config/manifest readers while preventing it from expanding into a broad serialization umbrella
update_note: `std.config` is the reusable parser layer for structured section/key documents with quoted strings, arrays, and inline-table field lookup; it now exposes a semantic keyed document model with stable order lists instead of a flat parser-entry bag, exists so `std.manifest` is not a second bespoke parser stack, and is still intentionally narrower than generic TOML/JSON/YAML/serde support
promotion_condition: Arcana-side tooling either keeps using the explicit config-document substrate or the module is intentionally relocated with the same narrow contract

id: STD-MANIFEST
classification: bootstrap-required
why: package manifest and lockfile parsing support for bootstrap tooling
consumers: package/build bootstrap path, future selfhost manifest readers
current_source: rewrite-owned
still_needs_rebuild: keep the Arcana-specific wrapper aligned with the active v4 package-id-aware lockfile contract while preserving explicit legacy v1 compatibility and without turning it back into a second generic parser stack
update_note: `std.manifest` now sits above `std.config` instead of carrying its own general parser helpers; keep it Arcana-specific and explicit, target `book.toml` plus `Arcana.lock` v4 while retaining `parse_lock_v1` as legacy compatibility, expose explicit manifest fields and lookup helpers rather than parser state, cover versioned local-registry dependency fields and source kinds, require active v4 lockfile core fields such as `workspace`, `workspace_root`, `[packages]`, `[dependencies]`, and `[builds."<package_id>"."<target>"]`, and revisit only if manifest parsing moves entirely behind the Rust driver before selfhost
promotion_condition: either rewrite-owned Arcana tooling still needs public manifest parsing or the module is relocated out of public `std`

id: STD-RESULT-OPTION
classification: bootstrap-required
why: core control/data carriers required across compiler, runtime, and examples
consumers: broadly across `std`, compiler grimoires, and examples
current_source: mixed
still_needs_rebuild: none beyond continued contract stability
update_note: keep these minimal and language-adjacent; explicit baseline methods such as `is_ok` / `is_err` / `unwrap_or` and `is_some` / `is_none` / `unwrap_or` are acceptable, but richer combinator-heavy policy should not be assumed by default
promotion_condition: already effectively stable once rewrite-owned toolchain consumes them without carried-only assumptions

id: STD-BYTES-TEXT
classification: bootstrap-required
why: source loading, tokenization, manifest parsing, and host-tool text handling
consumers: frontend/parser/typecheck, package/build tooling, owned host-core tools
current_source: mixed
still_needs_rebuild: keep byte-oriented UTF-8 helpers while preventing parser-specific convenience drift from becoming default std contract
update_note: the approved baseline now includes explicit search/trim/split/join/repeat/int-parse text helpers plus explicit bytes search/concat and `sha256_hex` helpers, but parser-specific or formatting-policy helpers still need review before entering std
promotion_condition: rewrite-owned toolchain uses a narrowed, explicitly justified text/bytes surface

id: STD-BINARY
classification: bootstrap-required
why: explicit binary parsing/emission substrate for font/image/audio/tooling work on top of memory views
consumers: future asset/codegen/runtime readers and owned tooling helpers
current_source: rewrite-owned
still_needs_rebuild: keep the surface narrow and explicit rather than turning it into a generic serialization framework
update_note: `std.binary` now provides the narrow reader/writer floor over `std.memory` byte views plus explicit opt-in codec hooks (`BinaryReadable`, `ByteSink`); keep it to seek/skip/remaining, endian-aware integer reads/writes, subview operations, and explicit codec traits only, with structured formats still living in domain libraries or explicit cabi/tooling contracts
promotion_condition: rewrite-owned toolchain and grimoires keep using the narrow binary-reader surface without pressure to broaden it into a generic format umbrella

id: STD-ITER-COLLECTIONS
classification: bootstrap-required
why: core data-structure support for compiler, runtime, and showcase corpus
consumers: compiler grimoires, selfhost examples, showcase and ECS examples
current_source: mixed
still_needs_rebuild: confirm which list/array/map/set helpers are true substrate versus carried convenience
update_note: explicit baseline ergonomics such as `is_empty`, list extension/clear, and `Map.get_or` are acceptable, but collection growth should still be justified by repeated substrate-level use rather than example-only ergonomics
promotion_condition: rewrite-owned compiler/runtime corpus uses a documented, stable collections subset

id: STD-MEMORY
classification: bootstrap-required
why: memory phrases, views, and allocator/publication semantics are part of the approved pre-selfhost baseline
consumers: memory examples, compiler/runtime paths, `std.binary`, and future higher-level text layers
current_source: mixed
still_needs_rebuild: preserve typed ownership/publication contract while hardening aliasing/runtime details under the approved family/view surface
update_note: arena/frame/pool plus temp/session/ring/slab, executable memory phrases, the builtin `View[Elem, Family]` surface (`Contiguous`, `Strided`, `Mapped`), borrowed-slice syntax, the owned payload families (`Bytes`, `ByteBuffer`, `Utf16`, `Utf16Buffer`), deterministic pool compaction/live-id behavior, current `seal` / `unseal` publication state for session/slab, and the allocator/publication capability layer (`Resettable`, `IdAllocating`, `LiveIterable`, `Compactable`, `SequenceBuffer`, `Sealable`) now sit on the rewrite backend; further allocator or borrow/resource-model expansion still goes through explicit scope/deferred docs
promotion_condition: rewrite-owned runtime fully supports approved memory surface and ownership rules

id: STD-CONCURRENT
classification: bootstrap-required
why: async/weave/split/channel/mutex/atomic runtime surface is already part of the frozen matrix
consumers: async examples, behavior examples, showcase demos
current_source: mixed
still_needs_rebuild: harden scheduler/worker semantics beyond the current deterministic eager task/thread lane without widening concurrency API surface
update_note: keep concurrency low-level; do not fold framework-level job systems into `std.concurrent`, and treat task/thread/channel/mutex/atomic runtime support as the rebuilt floor rather than the final scheduler design
promotion_condition: rewrite-owned runtime satisfies current async/concurrency examples and matrix coverage

id: STD-ECS-BEHAVIORS
classification: bootstrap-required
why: ECS scheduling/components and behavior stepping are first-party Arcana direction
consumers: owned ECS/behavior runtime, owned showcase layers, behavior examples
current_source: mixed
still_needs_rebuild: preserve first-party ECS/runtime surface while keeping broad query authoring out of the pre-selfhost baseline
update_note: `std.ecs`, `std.behaviors`, and `std.behavior_traits` stay first-party; revisit only to split layering more clearly, not to demote them
promotion_condition: rewrite-owned runtime supports approved ECS/behavior surface without relying on carried runtime assumptions

id: STD-TYPES-CORE
classification: bootstrap-required
why: shared low-level geometry/color/time/frame wrappers for retained substrate and toolchain-facing helpers
consumers: `std.time` and future Arcana-owned layers above the substrate
current_source: mixed
still_needs_rebuild: keep types low-level and substrate-oriented
update_note: new core types require concrete substrate consumers; gameplay/domain types do not automatically belong here
promotion_condition: rewrite-owned substrate uses a small stable core-type layer with documented purpose

id: STD-WINDOW
classification: removed-from-std
why: the public desktop shell no longer belongs in `std`
consumers: future non-std app-facing layers
current_source: removed-from-std
still_needs_rebuild: none in std; any replacement owner must be scoped separately
update_note: any remaining std window code is migration debt only; `std` is not the public desktop owner
promotion_condition: only revisit if a future explicit scope restores a public std desktop lane

id: STD-INPUT
classification: removed-from-std
why: public input handling no longer belongs in `std`
consumers: future non-std app-facing layers
current_source: removed-from-std
still_needs_rebuild: none in std; any replacement owner must be scoped separately
update_note: any remaining std input code is migration debt only; `std` is not the public input owner
promotion_condition: only revisit if a future explicit scope restores a public std desktop lane

id: STD-TEXT-INPUT
classification: removed-from-std
why: public text-input handling no longer belongs in `std`
consumers: future text/UI layers and showcase apps
current_source: removed-from-std
still_needs_rebuild: none in std; any replacement owner must be scoped separately
update_note: the public std text-input lane is retired; remaining text-input work lives outside `std`
promotion_condition: only revisit if a future explicit scope restores a public std desktop lane

id: STD-EVENTS
classification: removed-from-std
why: typed desktop event flow no longer belongs in `std`
consumers: future non-std app-facing layers
current_source: removed-from-std
still_needs_rebuild: none in std; any replacement owner must be scoped separately
update_note: any remaining std events code is migration debt only; `std` is not the public event owner
promotion_condition: only revisit if a future explicit scope restores a public std desktop lane

id: STD-CLIPBOARD
classification: removed-from-std
why: public clipboard handling no longer belongs in `std`
consumers: future text/UI layers and showcase apps
current_source: removed-from-std
still_needs_rebuild: none in std; any replacement owner must be scoped separately
update_note: the public std clipboard lane is retired; any replacement contract lives outside `std`
promotion_condition: only revisit if a future explicit scope restores a public std desktop lane

id: STD-CANVAS
classification: removed-from-std
why: the public software-present backend no longer belongs in `std`
consumers: future non-std graphics and text layers
current_source: removed-from-std
still_needs_rebuild: none in std; any replacement owner must be scoped separately
update_note: the public std canvas lane is retired; any replacement software-present backend lives outside `std`
promotion_condition: only revisit if a future explicit scope restores a public std graphics lane

id: STD-TIME
classification: bootstrap-required
why: low-level monotonic timing substrate for run-loop and frame-timing grimoires
consumers: owned showcase/runtime-smoke proofs and future non-std layers
current_source: rewrite-owned
still_needs_rebuild: runtime/backend implementation of monotonic clocks beyond the current compile-time substrate
update_note: keep `std.time` low-level; fixed-step or app-loop policy stays out of std
promotion_condition: rewrite-owned runtime implements the documented monotonic timing surface and showcase consumers build above it

id: STD-AUDIO
classification: removed-from-std
why: the public low-level audio lane now belongs to `arcana_audio`
consumers: owned audio-smoke proofs and future higher-level audio layers
current_source: removed-from-std
still_needs_rebuild: none in std; low-level audio ownership now sits in `arcana_audio`
update_note: the public std audio lane is retired; `AudioDevice`, `AudioBuffer`, and `AudioPlayback` remain active through `arcana_audio`, while higher-level playback policy stays in grimoires
promotion_condition: only revisit if a future explicit scope restores a public std audio lane

## Transitional-Carried

id: STD-APP
classification: transitional-carried
why: current examples use fixed-step helpers, but the module is convenience architecture rather than required substrate
consumers: no current consumers remain
current_source: removed-from-std
still_needs_rebuild: none before selfhost; any future reintroduction requires a fresh scope case
update_note: the carried fixed-step helpers were removed from `std` rather than promoted; desktop loop policy belongs in desktop app-shell grimoires
promotion_condition: only reintroduce if repeated cross-consumer evidence shows a minimal helper subset truly belongs in std

id: STD-TOOLING
classification: transitional-carried
why: local workspace planning helpers exist today, but they are toolchain convenience rather than general std substrate
consumers: example-local support only
current_source: removed-from-std
still_needs_rebuild: none before selfhost; keep tooling helpers out of std unless a future public support layer is explicitly scoped
update_note: the carried planning helper was relocated out of `std` into example-local support so std does not keep a toolchain-convenience surface by inertia
promotion_condition: only promote if a stable, intentionally public support layer is still needed outside the Rust driver

id: STD-TYPES-GAME
classification: transitional-carried
why: current game-flavored wrapper types exist in std, but they are domain-facing rather than clearly substrate-level
consumers: no current consumers remain
current_source: removed-from-std
still_needs_rebuild: none before selfhost; future showcase-specific wrappers should start outside std
update_note: the carried game-wrapper types were removed from `std`; the `std.types` root remains core-first and showcase/domain types must prove they belong elsewhere before promotion
promotion_condition: only promote if they become demonstrably necessary across multiple Arcana-owned grimoire layers, not just showcase examples

## Deferred

id: STD-DEFERRED-HIGHER-LAYERS
classification: deferred
why: higher-level desktop/game/audio/UI policies are intentionally outside the pre-selfhost std substrate
consumers: future Arcana-owned grimoire layers and post-selfhost ecosystem work
current_source: missing
still_needs_rebuild: none before selfhost
update_note: track in `docs/specs/std/std/deferred-roadmap.md`; do not reintroduce through carried std convenience layers
promotion_condition: post-selfhost or demonstrated pre-selfhost blocker approved through explicit scope update
