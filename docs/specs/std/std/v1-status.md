# Arcana Standard Library v1 Status

Status: `approved-pre-selfhost`

This ledger tracks bootstrap-readiness for the rewrite-owned `std` surface.

Rules:
- Every `std` surface change must update this ledger or `docs/specs/std/std/v1-scope.md` in the same patch.
- Transitional carried modules must include an update note that states what still needs to change before they can be treated as rewrite-owned.
- This ledger may classify or track `std`; it may not expand `std` surface by itself.
- Family entries cover their narrow wrapper/root modules unless noted otherwise. For example, collection-family status covers `std.collections.*` plus the thin `std.list` / `std.array` method wrappers.
- The current ledger is intended to support a pre-selfhost freeze of std shape: unless Milestone 6/runtime work or owned-grimoire development proves a real blocker, std should now move by implementation and verification rather than by repeated architectural redesign.

## Bootstrap-Required

id: STD-ARGS
classification: bootstrap-required
why: command-line bootstrap and host-tool entrypoint arguments
consumers: `grimoires/reference/examples/selfhost_host_tool_mvp`, `grimoires/reference/toolchain/arcana-selfhost-compiler`
current_source: mixed
still_needs_rebuild: keep surface narrow and host-root safe under the rewrite runtime boundary
update_note: keep argument access low-level; do not add parsing or CLI-framework helpers here
promotion_condition: host-core runtime is rewrite-owned and the public surface matches `selfhost-host/v1-scope.md`

id: STD-ENV
classification: bootstrap-required
why: environment access for host tools and bootstrap flows
consumers: host-tool and backend/runtime bootstrap lanes
current_source: mixed
still_needs_rebuild: preserve low-level environment lookups without growing policy helpers
update_note: add only concrete bootstrap-needed lookups; keep higher-level config loading out of `std.env`
promotion_condition: rewrite-owned host-core runtime satisfies documented env surface with no carried-only assumptions

id: STD-PATH
classification: bootstrap-required
why: workspace, artifact, and manifest path handling during bootstrap
consumers: `grimoires/reference/toolchain/arcana-frontend`, `grimoires/reference/toolchain/arcana-compiler-core`, `grimoires/reference/toolchain/arcana-selfhost-compiler`, `grimoires/reference/examples/selfhost_host_tool_mvp`
current_source: mixed
still_needs_rebuild: narrow convenience drift and align with approved host-core scope
update_note: keep only path substrate helpers; `is_absolute`, `stem`, `with_ext`, `relative_to`, `canonicalize`, and `strip_prefix` are now ratified as explicit end-user baseline helpers, but path policy helpers still do not belong in `std.path`
promotion_condition: public path surface matches approved host-core scope and rewrite-owned runtime behavior

id: STD-FS
classification: bootstrap-required
why: source loading, artifact writes, and host-tool IO during bootstrap
consumers: `grimoires/reference/toolchain/arcana-frontend`, `grimoires/reference/toolchain/arcana-compiler-core`, `grimoires/reference/toolchain/arcana-selfhost-compiler`, `grimoires/reference/examples/selfhost_host_tool_mvp`
current_source: mixed
still_needs_rebuild: narrow fallback-helper drift and align with approved host-core scope
update_note: keep filesystem APIs substrate-level; the carried fallback wrappers `read_text_or`, `list_dir_or_empty`, `mkdir_all_or_false`, `write_text_or_false`, `read_bytes_or_empty`, and `write_bytes_or_false` were removed from `std.fs`, explicit end-user helpers such as `create_dir`, `remove_dir`, `copy_file`, `rename`, `file_size`, and `modified_unix_ms` are now part of the approved host-core baseline, and stream APIs now use a typed `FileStream` handle instead of raw `Int` ids
promotion_condition: rewrite-owned host-core runtime satisfies approved fs surface and carried convenience drift is removed or explicitly ratified

id: STD-PROCESS
classification: bootstrap-required
why: host-tool execution status checks and controlled process capability gate
consumers: `grimoires/reference/examples/selfhost_host_tool_mvp`
current_source: rewrite-owned
still_needs_rebuild: keep the public surface contracted to approved status-only process execution unless new bootstrap need is documented
update_note: `std.process` now includes explicit capture helpers for end-user tools, but it must not re-expose compiler bootstrap escape hatches or drift into process-management framework policy
promotion_condition: process surface remains aligned with host-core scope and any future addition is bootstrap-scoped and documented

id: STD-IO
classification: bootstrap-required
why: text output for tools, diagnostics, and showcase-side proof consumers
consumers: compiler grimoires, host-tool examples, showcase examples
current_source: mixed
still_needs_rebuild: maintain low-level print semantics without turning `std.io` into a formatting framework
update_note: the approved baseline now includes stdout/stderr output, flush hooks, and explicit line input/output helpers, but higher-level logging and terminal UI policy should live in grimoires or tools, not `std.io`
promotion_condition: rewrite-owned runtime provides stable low-level output semantics for bootstrap and showcase consumers

id: STD-MANIFEST
classification: bootstrap-required
why: package manifest and lockfile parsing support for bootstrap tooling
consumers: package/build bootstrap path, future selfhost manifest readers
current_source: mixed
still_needs_rebuild: validate actual rewrite needs and align lockfile assumptions with the current repo contract
update_note: keep this toolchain-oriented and explicit; the current baseline now includes explicit helpers for workspace-member arrays, inline path deps, and lockfile order/deps/path/fingerprint/artifact lookups without promoting a generic TOML framework into std, and malformed lines or unterminated quoted strings now fail fast instead of being silently skipped; revisit if manifest parsing moves entirely behind the Rust driver before selfhost
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
consumers: `grimoires/reference/toolchain/arcana-frontend`, `grimoires/reference/toolchain/arcana-compiler-core`, `grimoires/reference/toolchain/arcana-selfhost-compiler`, host-tool examples
current_source: mixed
still_needs_rebuild: keep byte-oriented UTF-8 helpers while preventing parser-specific convenience drift from becoming default std contract
update_note: the approved baseline now includes explicit search/trim/split/join/repeat/int-parse text helpers plus explicit bytes search/concat and `sha256_hex` helpers, but parser-specific or formatting-policy helpers still need review before entering std
promotion_condition: rewrite-owned toolchain uses a narrowed, explicitly justified text/bytes surface

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
why: memory phrases and arena/frame/pool ownership model are part of the carried baseline
consumers: memory examples, showcase core, compiler/runtime paths that rely on arena-style ownership
current_source: mixed
still_needs_rebuild: preserve typed ownership contract while runtime/backend support is rebuilt
update_note: memory APIs are first-party direction, but future allocator families still go through explicit scope/deferred docs
promotion_condition: rewrite-owned runtime fully supports approved memory surface and ownership rules

id: STD-CONCURRENT
classification: bootstrap-required
why: async/weave/split/channel/mutex/atomic runtime surface is already part of the frozen matrix
consumers: async examples, behavior examples, showcase demos
current_source: mixed
still_needs_rebuild: align runtime implementation with the rewrite backend without widening concurrency API surface
update_note: keep concurrency low-level; do not fold framework-level job systems into `std.concurrent`
promotion_condition: rewrite-owned runtime satisfies current async/concurrency examples and matrix coverage

id: STD-ECS-BEHAVIORS
classification: bootstrap-required
why: ECS scheduling/components and behavior stepping are first-party Arcana direction
consumers: `grimoires/reference/examples/grimoire_ecs_schedule`, `grimoires/reference/examples/grimoire_ecs_mini_game`, `grimoires/reference/examples/topdown_arena_showcase`, behavior examples
current_source: mixed
still_needs_rebuild: preserve first-party ECS/runtime surface while keeping broad query authoring out of the pre-selfhost baseline
update_note: `std.ecs`, `std.behaviors`, and `std.behavior_traits` stay first-party; revisit only to split layering more clearly, not to demote them
promotion_condition: rewrite-owned runtime supports approved ECS/behavior surface without relying on carried runtime assumptions

id: STD-TYPES-CORE
classification: bootstrap-required
why: shared low-level geometry/color/time/frame wrappers for app/media substrate and toolchain-facing helpers
consumers: `std.canvas`, `std.time`, future Arcana-owned grimoire layers above the substrate
current_source: mixed
still_needs_rebuild: keep types low-level and substrate-oriented
update_note: new core types require concrete substrate consumers; gameplay/domain types do not automatically belong here
promotion_condition: rewrite-owned substrate uses a small stable core-type layer with documented purpose

id: STD-WINDOW
classification: bootstrap-required
why: raw window lifecycle/state/control substrate for desktop apps and showcases
consumers: `grimoires/owned/app/arcana-desktop`, window/input examples, showcase examples
current_source: carried
still_needs_rebuild: backend/runtime ownership under the rewrite, without inheriting old framework policy
update_note: keep only raw window substrate here; `open` is now explicitly fallible (`Result[Window, Str]`) and `alive` remains the lifecycle/pump edge, `std.canvas.open`/`alive` remain bootstrap compatibility wrappers, ergonomic desktop loops and policies belong in future Arcana-owned grimoire layers, and the current typed opaque `Window` handle is a bootstrap seam rather than a permanently ratified resource model
promotion_condition: rewrite-owned app/runtime backend implements the approved low-level window surface

id: STD-INPUT
classification: bootstrap-required
why: raw keyboard/mouse polling and code lookup for desktop apps and showcases
consumers: `grimoires/owned/app/arcana-desktop`, input and showcase examples
current_source: carried
still_needs_rebuild: backend/runtime ownership under the rewrite and documented event/input timing semantics
update_note: action mapping and richer input helpers belong in grimoires above `std.input`
promotion_condition: rewrite-owned input substrate satisfies the approved low-level surface

id: STD-EVENTS
classification: bootstrap-required
why: typed event queue and frame-pump boundary for desktop consumers
consumers: `grimoires/owned/app/arcana-desktop`, events demos, showcase examples
current_source: carried
still_needs_rebuild: confirm deterministic event pump semantics under the rewrite backend/runtime
update_note: routing, snapshots, and keybind helpers belong in grimoires above `std.events`; the public event surface is `Option`/`List`-based and now assumes a single backend event-record poll per step rather than separate kind/payload probes
promotion_condition: rewrite-owned event substrate and pump semantics are documented and tested

id: STD-CANVAS
classification: bootstrap-required
why: primitive render/text/image substrate for desktop apps and showcase proof
consumers: `grimoires/owned/app/arcana-graphics`, `grimoires/owned/app/arcana-text`, `grimoires/reference/examples/window_*`, showcase examples
current_source: carried
still_needs_rebuild: backend/runtime ownership under the rewrite and explicit primitive-graphics contract
update_note: keep canvas low-level; the approved independent-development baseline now includes line draw, filled circle draw, and default-font label measurement in addition to rect/text/image primitives, `open` stays as a bootstrap compatibility wrapper over `std.window.open`, `image_load` is now explicitly fallible (`Result[Image, Str]`), UI kits and richer scene/render abstractions belong in grimoires, and the current typed opaque `Image` handle is only a bootstrap seam until the rewrite-owned resource model is revisited
promotion_condition: rewrite-owned app/runtime backend satisfies primitive render/text/image surface

id: STD-TIME
classification: bootstrap-required
why: low-level monotonic timing substrate for run-loop and frame-timing grimoires
consumers: `grimoires/owned/app/arcana-desktop`, showcase run-loop consumers, runtime smoke demos
current_source: rewrite-owned
still_needs_rebuild: runtime/backend implementation of monotonic clocks beyond the current compile-time substrate
update_note: keep `std.time` low-level; fixed-step or app-loop policy stays out of std
promotion_condition: rewrite-owned runtime implements the documented monotonic timing surface and showcase consumers build above it

id: STD-AUDIO
classification: bootstrap-required
why: low-level audio output/buffer/playback substrate needed to support a later first-party audio grimoire
consumers: `grimoires/owned/app/arcana-audio`, `grimoires/reference/examples/audio_smoke_demo`
current_source: rewrite-owned
still_needs_rebuild: runtime/backend implementation of audio device/buffer/playback intrinsics
update_note: keep `std.audio` substrate-level; `default_output`, `buffer_load_wav`, and `play_buffer` are now explicitly fallible (`Result[...]`) while output lifecycle/info hooks plus pause/resume/looping/gain/position playback control remain baseline, mixing, streaming policy, and ergonomic playback helpers belong in grimoires, and the current typed opaque audio handles are bootstrap seams rather than long-term resource-model commitments
promotion_condition: rewrite-owned runtime provides documented low-level audio support and the first audio facade grimoire builds above it

## Transitional-Carried

id: STD-APP
classification: transitional-carried
why: current examples use fixed-step helpers, but the module is convenience architecture rather than required substrate
consumers: no current consumers remain
current_source: removed-from-std
still_needs_rebuild: none before selfhost; any future reintroduction requires a fresh scope case
update_note: the carried fixed-step helpers were removed from `std` rather than promoted; desktop loop policy belongs in facade grimoires
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
