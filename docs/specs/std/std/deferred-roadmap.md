# Standard Library Deferred Roadmap

Status: `authoritative-deferred-ledger`

This ledger tracks std work intentionally deferred from the pre-selfhost bootstrap plan.

Current std authority is:
- `docs/specs/std/std/v1-scope.md`
- `docs/specs/std/std/v1-status.md`
- `docs/specs/selfhost-host/selfhost-host/v1-scope.md`
- `docs/specs/selfhost-host/selfhost-host/app-substrate-v1-scope.md`

Guardrails:
- This ledger may schedule or classify follow-on std work, but it may not expand pre-selfhost std by itself.
- If a deferred item becomes a demonstrated bootstrap blocker, it must move through an explicit scope/status update instead of silently landing in code.
- Higher-level framework or gameplay surface should prefer Arcana-owned grimoire layers unless a concrete substrate-level need is proven.

id: STD-D1
title: richer text and font stack beyond primitive label draw
reason_deferred: the bootstrap path only requires primitive text draw; shaping, font families, glyph metrics, and rich layout are not yet substrate-critical.
target_window: post-first-runnable-backend app/runtime hardening
trigger_condition: ready_when=desktop app-shell grimoire and showcase consumers demonstrate repeated primitive-text limits; verify=text layout smoke coverage and deterministic render checks exist; blocked_by=no approved low-level text-resource contract.
owner: Arcana std/app-runtime team
acceptance_criteria: std exposes only the additional text/font substrate that higher-level grimoires actually need, without collapsing into a UI framework.
status: deferred

id: STD-D2
title: app/runtime resource-handle model review
reason_deferred: bootstrap can proceed with typed opaque `Window` / `Image` / `Audio*` handles, but the rewrite should not accidentally freeze Meadow-era resource-handle shape before the runtime/backend seam is fully rewrite-owned.
target_window: after the first runnable backend and before treating app/runtime handle shape as long-term stable
trigger_condition: ready_when=rewrite-owned runtime/backend resource seams exist; verify=the public handle model is evaluated against ownership, borrowing, interop, and backend ABI needs; blocked_by=current runtime/backend seams are still transitional.
owner: Arcana std/app-runtime team
acceptance_criteria: either the typed opaque handle model is explicitly re-ratified with rewrite-owned rationale or it is replaced/narrowed without leaking framework policy into std; the chosen model stays explicit, typed, and auditable, with no erased generic handle carrier or ambient capability surface.
status: deferred

id: STD-D3
title: higher-level audio engine and streaming support
reason_deferred: the pre-selfhost plan only needs low-level audio device, buffer, and basic playback substrate.
target_window: post-first-audio-grimoire stabilization
trigger_condition: ready_when=low-level `arcana_audio` and the first audio grimoire are stable; verify=audio smoke demos and grimoire parity checks pass; blocked_by=missing rewrite-owned backend audio substrate.
owner: Arcana std/audio team
acceptance_criteria: any std-side audio growth remains substrate-level and does not absorb mixer/playback policy better owned by grimoires.
status: deferred

id: STD-D4
title: gameplay/math/domain helper expansion
reason_deferred: gameplay helpers, broader math types, physics helpers, and domain-specific utility layers are not part of bootstrap std.
target_window: post-selfhost ecosystem shaping
trigger_condition: ready_when=multiple Arcana-owned grimoire layers or showcase consumers prove a repeated substrate-level gap; verify=the proposed helpers are not framework/domain convenience better owned by grimoires; blocked_by=no approved cross-domain contract.
owner: Arcana std/grimoire team
acceptance_criteria: only demonstrably substrate-level helpers graduate into std; the rest stay in Arcana-owned grimoire layers.
status: deferred

id: STD-D5
title: digest substrate beyond explicit SHA-256 helpers
reason_deferred: `std.text.bytes_sha256_hex` and the `Bytes` payload surface now cover the approved baseline, so the remaining question is only whether Arcana-side tooling later needs binary digests, incremental hashing, or other explicitly named digest helpers.
target_window: before any Arcana-owned package/build/fingerprint driver needs more than one-shot SHA-256 strings
trigger_condition: ready_when=Arcana-side tooling proves that `sha256_hex` is not enough for stable content/API fingerprints; verify=any added surface stays concrete, typed, deterministic, and narrowly named rather than turning into a broad crypto toolkit; blocked_by=current build/fingerprint logic still lives in the Rust rewrite.
owner: Arcana std/tooling team
acceptance_criteria: std exposes only the additional digest substrate Arcana-side tooling actually needs beyond `sha256_hex`, without collapsing into a broad crypto utility layer.
status: deferred

id: STD-D6
title: manifest and package substrate beyond explicit baseline helpers
reason_deferred: `arcana_process.path` now includes canonical path resolution and explicit prefix handling, `std.config` now provides the generic section/key parser substrate, and `std.manifest` now stays as a thin Arcana-specific wrapper over that substrate. The remaining deferred work is only whatever richer manifest/rendering support Arcana-side tooling later proves it still needs.
target_window: before any Arcana-owned package/workspace driver needs more than the current explicit path and manifest baseline
trigger_condition: ready_when=Arcana-side tooling needs richer manifest tables, typed lockfile rendering, or other deterministic package helpers beyond the current explicit lookup set; verify=the promoted helpers stay substrate-level and deterministic without turning `std.config` into a broad TOML/JSON/YAML/serde umbrella; blocked_by=the current package/build driver still lives in the Rust rewrite.
owner: Arcana std/tooling team
acceptance_criteria: std grows only the additional, narrowly named package/manifest pieces the Arcana-side package layer actually needs beyond the current baseline, while keeping `std.config` as a narrow deterministic config-document substrate rather than a broad serialization framework.
status: deferred

id: STD-D7
title: config document encapsulation and indexing review
reason_deferred: `std.config` is good enough for the pre-selfhost bootstrap contract, but its current semantic document model still exposes keyed storage and stable-order fields directly. If future Arcana-side tooling leans on it heavily, the next cleanup may be to hide more representation behind accessors or to add stronger indexing guarantees. That may also become irrelevant if later language/runtime systems give Arcana a better source-level document/opaque-data model.
target_window: post-selfhost tooling hardening
trigger_condition: ready_when=Arcana-side tooling uses `std.config` heavily enough that direct record-shape coupling or repeated document access becomes a maintenance or performance issue; verify=any redesign preserves deterministic ordering and fail-fast duplicate handling while reducing representation leakage; blocked_by=the current pre-selfhost contract intentionally prioritizes explicitness and simplicity over stronger encapsulation.
owner: Arcana std/tooling team
acceptance_criteria: either `std.config` is intentionally re-ratified as the stable semantic document shape, or it is narrowed behind a more explicit accessor/index model without reopening generic serialization scope; if later systems/features make the current record shape irrelevant, this item may be closed without direct std surgery.
status: deferred

id: STD-D8
title: manifest lookup/index model review
reason_deferred: `std.manifest` now has an explicit typed wrapper and correct lockfile parity for the current bootstrap contract, but its lookup tables are still list-backed and optimized for clarity rather than heavy package-driver use. If Arcana-side tooling later depends on it deeply, it may want a different indexed representation. This may also be superseded entirely if manifest/lock handling remains inside a different owned toolchain layer post-selfhost.
target_window: post-selfhost package/tooling ownership review
trigger_condition: ready_when=Arcana-side package/workspace tooling starts using `std.manifest` as a hot path rather than as an occasional deterministic helper; verify=any new lookup/index model stays explicit and Arcana-specific while keeping `book.toml` / `Arcana.lock` semantics stable; blocked_by=the Rust rewrite still owns the real package/build driver.
owner: Arcana std/tooling team
acceptance_criteria: either `std.manifest` remains a small explicit helper because heavier tooling lives elsewhere, or it gains a clearer indexed representation that removes repeated linear scans without turning into a second sprawling package driver; later systems/features may make this item unnecessary.
status: deferred

id: STD-D9
title: remaining builtin-family migration review
reason_deferred: runtime/resource handles have been moved out of the Rust builtin registry, but concurrency and memory families still remain compiler-known builtins. That is acceptable for the current roadmap, but later language features such as broader opaque/source-declared type support or different ownership/runtime systems may make another migration desirable or make the issue disappear on its own.
target_window: post-selfhost language/runtime cleanup
trigger_condition: ready_when=the selfhost compiler and runtime are stable enough to revisit the remaining builtin families without destabilizing the core roadmap; verify=any migration reduces compiler special-casing and keeps ownership/boundary behavior explicit; blocked_by=the current language still intentionally reserves some builtin families at the compiler level.
owner: Arcana language/runtime team
acceptance_criteria: either the remaining builtin families are intentionally re-ratified with rewrite-owned rationale, or they move behind a more general source-level mechanism without reintroducing ambiguity or special-case drift; if later systems/features make the current builtin boundary irrelevant, this item may be retired.
status: deferred
