# Arcana Temporary Review

Status: static review in progress; resolved crate, process, and winapi event-layer findings removed after recheck, remaining grimoire and architecture findings retained

Method:
- Read-only review
- No source changes
- No test reruns in this pass
- Review authority followed: `POLICY.md`, approved specs, then `crates/*`, then `llm.md` and first-party grimoires

Coverage Targets:
- `arcana-syntax` (`6` files, about `13k` lines)
- `arcana-frontend` (`9` files, about `25k` lines)
- `arcana-hir` (`7` files, about `10k` lines)
- `arcana-ir` (`6` files, about `11k` lines)
- `arcana-runtime` (`31` files, about `45k` lines)
- `arcana-package` (`8` files, about `14.5k` lines)
- `arcana-aot` (`18` files, about `15.8k` lines)
- `arcana-cli` (`9` files, about `3.3k` lines)
- `arcana-cabi` (`2` files, about `2.4k` lines)
- `grimoires/arcana/winapi` (`57` files, about `6.9k` lines)
- `grimoires/arcana/process` (`8` files, about `147` lines)
- `grimoires/libs/*` (`0` files in current workspace)

Coverage Status:
- `arcana-runtime`: partially audited
- `arcana-package`: partially audited
- `arcana-cli`: partially audited
- `arcana-aot`: partially audited
- `arcana-frontend`: partially audited
- `arcana-hir`: partially audited
- `arcana-cabi`: partially audited
- `arcana-syntax`: spot-audited, no concrete defect logged yet
- `arcana-ir`: spot-audited, no concrete defect logged yet
- `grimoires/arcana/winapi`: partially audited; latest event/frame cleanup rechecked and trimmed from active findings
- `grimoires/arcana/process`: partially audited; latest process-plan findings rechecked and removed
- `grimoires/libs/*`: no files present in current workspace

Findings:

Resolved crate/package/runtime/frontend/process and winapi event-layer findings that are actually fixed in the current tree were removed from this file after rechecking the code. Remaining active findings:

1. Medium - `docs/specs/resources/resources/v1-scope.md:16-26`, `grimoires/arcana/winapi/src/helpers/audio.arc:43-56`, `grimoires/arcana/winapi/src/helpers_audio_impl.arc:212-224,236-287`, and `grimoires/arcana/winapi/src/shackle.arc:119,301-307,381-391`
`AudioBuffer` has allocation and use but no normal teardown path. `buffer_load_wav` inserts a handle-backed buffer record into `audio_buffers`, the public API exposes read/use operations over that handle, and nothing removes the record during ordinary program execution. `AudioDevice` has `output_close`; `AudioPlayback` has `playback_stop`; `AudioBuffer` has nothing. The only actual cleanup path is whole package-state destruction when the binding unloads. That is hidden resource retention presented as if it were an ordinary owned handle family.

Related info:
- The approved resource scope requires lifecycle rules to be explicit and diagnosable. "Lives until package unload" is not an explicit public rule here; it is just what falls out of the missing teardown surface.
- `AudioBuffer` is declared as a `move` opaque handle family, which strongly suggests ordinary ownership semantics rather than immortal package-global residency.
- This is inconsistent even within the same audio package slice. Devices and playbacks have consuming operations; buffers do not, despite being stored in the same long-lived package-state map.

Possible solutions for correctness:
- Add an explicit consuming `buffer_destroy(take buffer: AudioBuffer)` / `buffer_close(...)` operation and remove the buffer from `audio_buffers` there.
- Or implement the standard cleanup contract for `AudioBuffer` so scope-exit cleanup can reclaim it.
- If buffers are intentionally meant to be package-lifetime shared assets, stop modeling them as opaque owned handles and document the real sharing/lifetime model explicitly instead of leaving it implicit in the map implementation.

2. Low - `llm.md:934,961-964`, `docs/arcana-v0.md:840-841`, `docs/specs/os-bindings/os-bindings/v1-scope.md:48,80`, `grimoires/arcana/winapi/src/types.arc:1-3`, `grimoires/arcana/winapi/src/foundation.arc:1-5`, `grimoires/arcana/winapi/src/fonts.arc:1-8`, and `grimoires/arcana/winapi/src/windows.arc:1-14`
The `arcana_winapi` migration-wrapper story is internally inconsistent. `llm.md` and the archival docs still describe typed compatibility wrappers like `arcana_winapi.types.ModuleHandle` and `arcana_winapi.types.HiddenWindow`, but the checked-in wrapper modules mostly export raw `HMODULE`, `HWND`, and naked `U64` values instead. Meanwhile `types.arc` declares opaque wrapper types that the public wrapper routines do not actually use. This is source/docs drift, not a new approved contract change, but it is still bad public surface hygiene because it leaves consumers with two contradictory type stories for the same wrapper layer.

Related info:
- The approved OS-binding scope only says compatibility wrappers remain available during migration. It does not bless two inconsistent wrapper type stories in parallel.
- The canonical handle families used by real higher-level grimoires (`desktop_handles`, `graphics_handles`, `process_handles`, `audio_handles`) are much more coherent than this compatibility wrapper lane.
- Because `llm.md` is supposed to be the quick guide for source work, stale typed-wrapper examples here are not harmless trivia. They actively steer review or implementation work toward APIs that the current grimoire source no longer exports.

Possible solutions for correctness:
- Pick one wrapper story and make the docs and source agree.
- If the typed wrapper lane is still intended, convert the wrapper routines to use `arcana_winapi.types.*` consistently.
- If the raw wrapper lane is the real surviving migration shape, remove or clearly deprecate the unused `types.arc` wrapper types and fix `llm.md` / archival examples so they stop promising nonexistent typed wrappers.

**Architecture Review**
This section is separate from the correctness findings above. These are maintainability and architecture findings: duplicated surfaces, conflicting semantic owners, hidden coupling, and code organization that will keep producing bugs even when the current tests are green.

A1. High - `grimoires/arcana/winapi/src/book.arc:1-6`, `grimoires/arcana/winapi/src/helpers.arc:1-16`, `grimoires/arcana/winapi/src/types.arc:1-3`, `grimoires/arcana/winapi/src/foundation.arc:1-5`, `grimoires/arcana/winapi/src/fonts.arc:1-8`, and `grimoires/arcana/winapi/src/windows.arc:1-14`
`arcana_winapi` exposes too many overlapping public lanes for the same conceptual surface. There is the raw lane, the helper lane, the canonical `*_handles` lane, and the compatibility-wrapper lane, and the wrappers are reexported both at package root and again under `helpers`. That means one concept can be reached through multiple routes with different type stories and different levels of abstraction. Even before correctness bugs enter the picture, this is a maintenance tax on every change because docs, tests, call sites, and migration stories all have to answer "which lane is the real one?" The current wrapper types in `types.arc` make this worse because they exist, are exported, and still are not the types actually used by the wrapper routines.

Related info:
- Several correctness findings above are downstream symptoms of this lane overlap rather than isolated accidents. The wrapper/doc drift in finding 2 and the resource-lifetime defect in finding 1 are both easier to create because there is no single clearly-owned public lane.
- The same concept can currently be reached through raw bindings, helper wrappers, compatibility wrappers, and canonical handle families. That encourages different callers to normalize on different lanes and makes "API drift" a routine outcome rather than an exceptional bug.
- High-quality platform bindings usually expose one canonical safe layer plus clearly-marked low-level escape hatches. Rust's standard library and mature FFI crates do not normally present four peer public routes for the same conceptual operation without a strong boundary story.

Possible solutions for correctness:
- Define one canonical Arcana-facing Win32 surface per concern and make every other lane explicitly secondary: either raw-only escape hatch, compatibility-only migration surface, or deprecated wrapper path.
- Add a package-surface policy note to `arcana_winapi` that every newly exported item must declare which lane it belongs to and why. If it cannot be classified, it should not be exported yet.
- Where compatibility wrappers remain, make their type story mechanically consistent with the chosen canonical lane so callers cannot silently mix two different ownership or typing models for the same operation family.

Possible solutions for maintainability:
- Choose one canonical public route per concern and demote the rest to explicit migration-only surface.
- Stop reexporting compatibility wrappers under `helpers`; that doubles the confusion for no good reason.
- Either make `types.arc` the real typed wrapper lane or remove/deprecate it instead of carrying dead-looking exported surface.

A2. Medium - `docs/specs/std/std/v1-scope.md:22-26`, `grimoires/arcana/winapi/src/helpers.arc:1-15`, `grimoires/arcana/winapi/src/shackle.arc:110-121,372-383`, `grimoires/arcana/winapi/src/helpers/window.arc:26-31,108-112`, `grimoires/arcana/winapi/src/helpers/audio.arc:7-18`, and `grimoires/arcana/winapi/src/helpers/clipboard.arc:4-12`
The `arcana_winapi.helpers` package shape is doubled into raw calls plus a package-global error slot plus Arcana-side `Result` wrappers. That is three layers of API shape for one operation family. It is backend-shaped leakage turned into public source structure. Every helper module repeats the same pattern: call a raw function, inspect `take_last_error`, then rebuild a typed result. That multiplies boilerplate, spreads the same error-translation logic across modules, and makes refactors expensive because one conceptual operation is represented in multiple layers instead of one.

Related info:
- The package-global `last_error_text` slot introduces sequencing coupling between otherwise unrelated operations. It is a transport concern being exposed as if it were a normal public helper idiom.
- The repeated helper pattern is mostly re-stating one transport policy over and over: invoke raw callback, then translate one string error channel into `Result`.
- High-quality language bindings usually keep raw transport/error conventions inside the FFI seam and expose one typed error surface to language-level callers. They do not require every helper family to hand-roll the same reconstruction logic.

Possible solutions for correctness:
- Collapse public Arcana-facing helper APIs onto one `Result`-returning layer so callers do not have to reason about global error-slot sequencing at all.
- Keep the raw error transport private to the binding glue, or if it must remain visible, mark it as explicit low-level compatibility surface rather than peer public API.
- Centralize error translation in one typed wrapper path so changes to error semantics cannot drift across process/window/audio/clipboard helpers independently.

Possible solutions for maintainability:
- Keep out-of-band error state inside binding glue only, not as a repeated public helper pattern.
- Export one canonical `Result`-returning helper layer for Arcana source consumers.
- Remove public raw-plus-wrapper duplication unless the raw form is explicitly needed as a separate supported surface.

A3. Medium - `grimoires/arcana/winapi/src/shackle.arc:109-121,305-325,372-383` and `grimoires/arcana/winapi/src/helpers_desktop_impl.arc:126-134,829-899,2115-2200`
`arcana_winapi` still hangs too much unrelated behavior off one package-state object. The same hidden state bucket owns the helper error slot, desktop/window state, wake objects, file streams, software surfaces, and audio devices/buffers/playbacks. That is still a classic god-object architecture even after the event/frame cleanup. It creates coupling between domains that should be able to evolve independently, and it makes extraction harder because teardown and state ownership are already braided together.

Related info:
- This "one state bucket" shape is a major reason new features keep getting added here. Once one package-state object already owns everything, the local path of least resistance is always "put one more map in it."
- Cross-domain teardown and failure handling become coupled by default in this shape. A change in audio/resource cleanup, wake/message handling, or stream lifetime is harder to reason about because all of it is braided through one shared owner.
- Major-quality runtime/binding layers usually organize resource ownership by subsystem with a small root coordinator, not one ever-growing struct that owns unrelated OS and helper concerns together.

Possible solutions for correctness:
- Split state ownership by domain so each resource family has a bounded lifecycle surface and explicit teardown rules.
- Keep the root package state tiny and make it point to domain-specific state objects rather than being the direct owner of every map.
- When one helper family needs another, require explicit access boundaries instead of shared reach into one global mutable bucket. That makes cross-domain invariants reviewable.

Possible solutions for maintainability:
- Split package state by domain rather than treating one giant struct as the default place for every helper-owned resource.
- Give each major subsystem its own teardown path and narrower internal contract.
- Make cross-domain access explicit instead of letting every helper family reach into one shared bucket by default.

A4. Medium - `crates/arcana-aot/src/instance_product.rs:1593-2105`
The AOT binding generator is still doing a large amount of runtime-support emission as hand-built source strings inside one mixed-role Rust file. Binding ABI semantics, generation policy, and the emitted runtime-support implementation are all tied together in one place. That is hard to read, hard to review, and hard to refactor safely because changing a runtime-support rule means editing stringified Rust embedded in the generator rather than ordinary typed helper code. The current dirty worktree is a good example: the input/output layout split is real progress, but it still has to thread through this string-built support slab instead of a smaller typed boundary.

Related info:
- The callback layout transport bug that was just fixed is a direct example of the cost of this structure. One ABI-direction rule had to be reasoned about across generator policy, emitted support code, runtime transport, and CABI meaning.
- Substring-oriented generation checks are much weaker than typed reuse. Even when the generated output is "correct today," it is harder to mechanically prove that several protocol directions stayed aligned.
- Major-quality generators usually keep protocol meaning in typed helper code or templates with narrow parameters, and reserve string concatenation for final rendering rather than for core semantic rules.

Possible solutions for correctness:
- Move binding transport rules out of large ad hoc string slabs and into typed helper APIs that the generator calls deliberately for each direction and value kind.
- Add one shared binding-transport conformance corpus exercised across `arcana-cabi`, `arcana-runtime`, and generated support so ABI rules are validated once, not rediscovered by review.
- Reduce the emitted support layer to small glue code that calls shared helpers rather than re-expressing protocol rules inline in generated source.

Possible solutions for maintainability:
- Move binding runtime support into typed helper modules or templates with narrower emit points.
- Split generation policy from emitted support implementation so ABI review and codegen review are not welded together.
- Keep the generated support layer minimal and reuse shared Rust helpers wherever possible.

A5. High - `crates/arcana-frontend/src/lib.rs:10-16,21-54,5092-5116,7059,8151,10177,18294-18301,20027-20060`
`arcana-frontend` is not acting like a frontend crate with clear stages. It is a very large `lib.rs` that owns public check/load entry points, semantic validation, executable foreword execution and caching, and then a giant integration-test slab in the same file. Only a handful of leaf helpers were split into modules; the real behavior is still piled into one compilation unit. That makes language semantics, adapter tooling, and test scaffolding churn together. Every change is harder to isolate because the crate does not express its own phase boundaries in code organization.

Related info:
- Several numbered findings above already terminate inside `arcana-frontend`, but the current structure makes it harder to tell which phase actually owns the defect: parsing/lowering, semantic validation, package identity handling, or adapter execution/caching.
- Giant single-file ownership increases compile noise and review noise. Small mechanical edits in one subsystem create unrelated merge/conflict pressure in every other subsystem that shares the file.
- Major-quality compiler frontends normally split parse/load, name and graph resolution, semantic validation, diagnostics, and auxiliary tooling into separate modules or crates. That is not style vanity; it is how semantic ownership stays legible.

Possible solutions for correctness:
- Separate phase owners in code so every semantic rule has one obvious home. That reduces the risk of one bug fix accidentally changing a different phase's assumptions.
- Move executable foreword materialization/caching behind a narrower subsystem boundary so language-semantics changes do not implicitly alter adapter execution behavior.
- Move the giant inline integration-test slab into dedicated test modules/files. When tests and production phase logic share one huge file, stale expectations and phase leakage are easier to miss.

Possible solutions for maintainability:
- Split workspace ingestion/check API, semantic validation, executable foreword tooling, diagnostics rendering, and tests into separate modules with one owner each.
- Stop letting `lib.rs` accumulate new subsystems just because they are "frontend-adjacent".
- Give the foreword adapter subsystem its own narrower crate or at least its own internal module tree and test tree.

A6. High - `crates/arcana-runtime/src/lib.rs:878-968,5788-5805,6209-6215,6294-6448,6631-6640,6702-6710,6743-6755`
`arcana-runtime` still routes host-core behavior through a giant string-key dispatcher in `lib.rs`, with broad shared execution state hanging off one runtime context. Filesystem and process behavior are selected by matching textual callable keys, then mutating shared runtime host/process/input state through the same central switchboard. That is an interpreter-shaped implementation detail leaking into long-term architecture. It is hard to search, hard to audit for completeness, and it invites unrelated host features to accumulate in one switchboard instead of in typed domain modules with narrow contracts.

Related info:
- Recent host-core bugs were harder to localize precisely because behavior lives in one dispatcher rather than in typed subsystem owners. A search for one host operation yields string tokens and fallback paths rather than one obvious implementation boundary.
- String-key dispatch weakens exhaustiveness. It is easy to add a new host callable in one place and forget a related helper, identity path, or validation path somewhere else because the compiler sees "string constant," not "missing enum arm."
- Mature runtimes often begin with text-key dispatch at the interpreter boundary, but they usually converge on typed operation tables or domain modules once the host surface becomes stable enough to matter semantically.

Possible solutions for correctness:
- Keep string names only at the interpreter boundary, then resolve them once into typed host operations or domain-dispatched tables before executing behavior.
- Split runtime state by host subsystem so file streams, process state, adapters, and other features stop sharing one default mutable home.
- Add subsystem-level conformance tests around typed operations rather than relying on giant end-to-end dispatcher coverage for semantic guarantees.

Possible solutions for maintainability:
- Split host subsystems by domain and dispatch through typed tables or enums instead of giant string matches.
- Keep runtime state per subsystem rather than treating one execution-state object as the default home for every host feature.
- Move host-core behavior behind explicit module boundaries so adding one syscall family does not require editing the same giant dispatcher.

A7. High - `docs/specs/os-bindings/os-bindings/v1-scope.md:53-66`, `crates/arcana-cabi/src/lib.rs:642-672,960-984,1322-1365,1675-1710`, `crates/arcana-runtime/src/binding_transport.rs:522-546,1049-1100,1663-1788,2170-2245`, and `crates/arcana-aot/src/instance_product.rs:1593-1655,1746-1830,2050-2195`
The binding ABI has no single semantic owner across the crate graph. `arcana-cabi` defines the canonical view/value/layout contract, `arcana-runtime` reimplements large parts of the same marshalling and validation logic, and `arcana-aot` emits another implementation as stringified Rust support code. That is not layered reuse; it is triplication. The recent input/output layout split is exactly the sort of bug this architecture manufactures, because each lane can quietly make a different assumption about the same protocol and no crate is obviously "the one true source".

Related info:
- The callback layout transport bug is the concrete proof here, not a hypothetical. The three owners had already drifted enough that callback input and output shape disagreed across crates before the last fix.
- `arcana-cabi` already occupies the natural semantic-owner role. It defines the layout and transport structures that the other crates are supposed to honor.
- Major-quality ABI layers centralize protocol meaning in one place and make other layers reuse it. When three layers each "understand" the ABI separately, correctness review turns into diffing interpretations instead of checking one contract.

Possible solutions for correctness:
- Make `arcana-cabi` the only semantic owner of binding-transport meaning and require runtime/AOT support to call into shared helpers or generated tables derived from that one owner.
- Add cross-crate conformance tests over one canonical binding matrix: scalars, opaques, layout values, strings, bytes, views, owned outputs, callback inputs, and callback outputs.
- Treat any crate-local transport reinterpretation as suspect unless it can be mechanically traced back to the canonical CABI contract. That is the only way to stop protocol drift from recurring.

Possible solutions for maintainability:
- Make `arcana-cabi` the single semantic owner of binding transport rules and push reuse outward from there.
- Keep runtime transport code thin and typed, not a parallel implementation of the ABI contract.
- Keep AOT support emission declarative and minimal, reusing shared helpers instead of restating protocol rules in generated strings.

A8. High - `crates/arcana-hir/src/lib.rs:1509-1525,1643-1659,2726-2743,3531-3700` and `crates/arcana-frontend/src/lib.rs:12129-12139,12578-12586,13135-13136`
The package identity model is still not singular at the crate boundary. `arcana-hir` stores both `package_names -> ids` and definitive `package_id` maps, exposes both `package(...)` and `package_by_id(...)`, and resolution artifacts carry both `package_id` and `package_name`. The frontend then bounces between the two lookup styles inside semantic code. That guarantees recurring ambiguity bugs because the architecture keeps telling callers that name-based lookup is a normal internal tool rather than a lossy parse/display convenience.

Related info:
- The repo already had concrete duplicate-name fallout here, and the recent fixes only make the architectural problem clearer: the implementation has enough identity information to be correct and still keeps lossy post-resolution lookup APIs alive.
- `package_by_id(...)` already exists and is already used elsewhere, so the repo is not blocked on missing capability. The current problem is that the architecture still treats name-based lookup as respectable after resolution.
- Compiler infrastructure in Rust/Swift/Clang-class toolchains separates user-facing names from semantic identities after resolution. Display names remain useful for source and diagnostics, but semantic work uses resolved identities.

Possible solutions for correctness:
- Demote `package(...)`-style name lookup to explicit pre-resolution or display-only utilities and stop presenting it as a normal post-resolution helper.
- Where both name and id remain present in resolved artifacts, make the id the semantically authoritative field and treat the name as cached display data only.
- Deprecate or rename lossy name-based helpers so their risk is obvious to callers. If an API can silently become ambiguous in a legal graph, it should not look like a general-purpose semantic lookup.

Possible solutions for maintainability:
- Standardize on `package_id` for all internal semantic lookup after parse/lowering.
- Restrict name-based lookup to explicit parse or display boundaries, not deep semantic helpers.
- Replace ambiguous helpers like `package(...)` with APIs that force callers to handle duplicate-name cases explicitly.

A9. Medium - `crates/arcana-package/src/lib.rs:1-44,108-121,1544-1557,1761-1815`, `crates/arcana-cli/src/main.rs:7-12`, and `crates/arcana-cli/src/package_cmd.rs:3-10,36-49,66-80`
`arcana-package` is a junk-drawer crate. It reexports build planning and execution, distribution staging, fingerprinting, publish, versioning, and lockfile I/O while also owning workspace HIR loading and support-file path policy. Then `arcana-cli` adds another orchestration layer in `package_cmd` that still has to do graph load, frontend check, build preparation, planning, execution, and bundle staging. That is too much responsibility spread across two crates and a command shim. When workspace ingestion, packaging policy, and distribution staging all live under the same umbrella, package changes become harder to reason about and harder to test in isolation.

Related info:
- Several package correctness findings are harder to localize because responsibility is split between package internals and CLI orchestration. The same end-to-end behavior can be influenced by distribution helpers, build identity policy, and command-layer staging decisions.
- Workspace HIR loading inside the packaging crate keeps packaging coupled to frontend-adjacent concerns. That makes package bugs and graph/loading bugs harder to separate mentally and in tests.
- Mature build/distribution stacks usually separate graph loading, build planning, artifact caching, staging/distribution, and CLI orchestration into clearer layers even if they still live in the same repository.

Possible solutions for correctness:
- Separate "what is the resolved package graph" from "how do we build/stage/distribute it" so cache and staging bugs can be reasoned about against stable graph/build inputs.
- Make the CLI a thin caller over stable packaging APIs rather than a second place where graph-loading and staging policy get recomposed.
- Consider a dedicated orchestration layer or crate for end-to-end packaging commands so `arcana-package` can narrow to build/distribution correctness instead of also owning frontend-adjacent loading and command assembly.

Possible solutions for maintainability:
- Pull workspace graph/HIR ingestion out of `arcana-package` so packaging stops owning frontend-adjacent loading concerns.
- Narrow `arcana-package` to build, distribution, and artifact policy instead of "everything around shipping".
- Keep `arcana-cli` as a thin coordinator over stable crate APIs rather than a second place where packaging workflow gets reassembled.

Correctness direction:
- For the remaining two correctness items, the standard to aim for is not merely "does not obviously crash in happy-path tests". The standard should be one of:
  - memory and type safety at foreign boundaries comparable to Rust expectations
  - cache and build reproducibility comparable to major build systems
  - filesystem behavior that is explicit, deterministic, and safe on Windows path semantics

Notes:
- No crate is marked as a literal every-line-complete audit yet. Coverage above reflects what has actually been inspected, not what would be convenient to claim.
- Resolved crate/package/runtime/frontend/process and winapi event-layer findings that are actually fixed in the current tree were removed from this file after recheck.
- Finding 1 is an implementation defect and approved-contract violation against the current approved resource scope.
- Finding 2 is source/docs drift in the grimoire layer, not a silent approved-contract change.
- This pass prioritized `crates/*` and runtime/package/AOT seams over examples and fixtures, per repo review policy.
- `grimoires/arcana/winapi` and `grimoires/arcana/process` are now in the active audit set; `grimoires/libs/*` currently has no files in this workspace.
- `grimoires/arcana/process` was rechecked after the process-plan work and no longer has an active public-surface finding in this file.
- `grimoires/arcana/winapi` was rechecked after the `winfix.md` cleanup and the old public event/frame findings were removed from this file.
- No code was changed.
