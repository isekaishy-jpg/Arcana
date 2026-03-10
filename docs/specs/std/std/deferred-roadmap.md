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
- Higher-level framework or gameplay surface should prefer first-party grimoires unless a concrete substrate-level need is proven.

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
trigger_condition: ready_when=multiple first-party grimoires or showcase consumers prove a repeated substrate-level gap; verify=the proposed helpers are not framework/domain convenience better owned by grimoires; blocked_by=no approved cross-domain contract.
owner: Arcana std/grimoire team
acceptance_criteria: only demonstrably substrate-level helpers graduate into std; the rest stay in first-party grimoires.
status: deferred
