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
trigger_condition: ready_when=desktop facade grimoire and showcase consumers demonstrate repeated primitive-text limits; verify=text layout smoke coverage and deterministic render checks exist; blocked_by=no approved low-level text-resource contract.
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
target_window: post-first-audio-facade stabilization
trigger_condition: ready_when=low-level `std.audio` and the first audio facade grimoire are stable; verify=audio smoke demos and facade parity checks pass; blocked_by=missing rewrite-owned runtime audio backend.
owner: Arcana std/audio team
acceptance_criteria: any std-side audio growth remains substrate-level and does not absorb mixer/facade policy better owned by grimoires.
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
reason_deferred: `std.bytes.sha256_hex` is now part of the approved baseline, so the remaining question is only whether Arcana-side tooling later needs binary digests, incremental hashing, or other explicitly named digest helpers.
target_window: before any Arcana-owned package/build/fingerprint driver needs more than one-shot SHA-256 strings
trigger_condition: ready_when=Arcana-side tooling proves that `sha256_hex` is not enough for stable content/API fingerprints; verify=any added surface stays concrete, typed, deterministic, and narrowly named rather than turning into a broad crypto toolkit; blocked_by=current build/fingerprint logic still lives in the Rust rewrite.
owner: Arcana std/tooling team
acceptance_criteria: std exposes only the additional digest substrate Arcana-side tooling actually needs beyond `sha256_hex`, without collapsing into a broad crypto utility layer.
status: deferred

id: STD-D6
title: manifest and package substrate beyond explicit baseline helpers
reason_deferred: `std.path` now includes canonical path resolution and explicit prefix handling, `std.config` now provides the generic section/key parser substrate, and `std.manifest` now stays as a thin Arcana-specific wrapper over that substrate. The remaining deferred work is only whatever richer manifest/rendering support Arcana-side tooling later proves it still needs.
target_window: before any Arcana-owned package/workspace driver needs more than the current explicit path and manifest baseline
trigger_condition: ready_when=Arcana-side tooling needs richer manifest tables, typed lockfile rendering, or other deterministic package helpers beyond the current explicit lookup set; verify=the promoted helpers stay substrate-level and deterministic without turning `std.config` into a broad TOML/JSON/YAML/serde umbrella; blocked_by=the current package/build driver still lives in the Rust rewrite.
owner: Arcana std/tooling team
acceptance_criteria: std grows only the additional, narrowly named package/manifest pieces the Arcana-side package layer actually needs beyond the current baseline, while keeping `std.config` as a narrow deterministic config-document substrate rather than a broad serialization framework.
status: deferred
