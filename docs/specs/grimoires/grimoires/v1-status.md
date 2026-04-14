# Arcana Grimoire Roles v1 Status

Status: `approved-pre-selfhost`

This ledger tracks bootstrap-readiness for required future Arcana-owned app/media grimoire roles.

Rules:
- Every Arcana-owned grimoire role change must update this ledger or `docs/specs/grimoires/grimoires/v1-scope.md` in the same patch.
- Carried roles must include an update note explaining what still needs to be rebuilt or replaced before they can be treated as rewrite-owned.
- This ledger may classify required app/media roles; it may not expand the required grimoire set by itself.
- Archived historical MeadowLang corpus is bootstrap context only, not future grimoire architecture.
- Generic OS-binding grimoires such as `arcana_winapi` are tracked separately under the OS-binding scope rather than this app/media ledger.

id: GRIMOIRE-DESKTOP-APP-SHELL
classification: bootstrap-required
role: desktop/app-shell grimoire
current_package: `grimoires/libs/arcana-desktop`
historical_seed: archived MeadowLang desktop app corpus
why: Arcana-owned public desktop/window/event-loop boundary for native desktop apps above the low-level app/runtime substrate
current_source: rewrite-owned-in-progress
still_needs_rebuild: broaden the current real session runner/mailbox/window layer into the full long-term desktop contract, with remaining parity work focused on additional low-level window config knobs and richer input metadata only where future grimoires prove those belong in shared `std.*` substrate rather than desktop-only shortcuts; keep raw app-shell power in `arcana_desktop` itself instead of splitting the public boundary across a weaker wrapper plus raw `std.*`
update_note: the scaffold is now replaced by a real rewrite-owned app-shell package with static `Application[...]` runner callbacks, session-backed window orchestration, blocking wait support, configurable wait slicing, wake/mailbox helpers, monitor wrappers, clipboard wrappers, settings-facing window and text-input records, cursor/theme hooks, richer key metadata and composition-event routing, and native-handle export for graphics backends. The intended end state is a winit-class desktop shell plus clipboard, with canvas, loop, and ECS helpers retired from the public desktop surface. This package is the canonical public desktop boundary and should consume `arcana_winapi` rather than preserve the old public `std.*` desktop shell as a parallel app-facing API. The current runner contract is window-ID centric in the winit sense; `main_window` remains only a runner convenience and, after queued close reconciliation, may promote a surviving live window rather than pinning the original launch window forever.
promotion_condition: the rewrite-owned desktop/app-shell package reaches the approved long-term role breadth and powers native showcase apps as the normal public shell boundary, with WinAPI-bound substrate debt moved out of runtime/std and into grimoires

id: GRIMOIRE-GRAPHICS
classification: bootstrap-required
role: graphics grimoire
current_package: `grimoires/libs/arcana-graphics`
historical_seed: archived MeadowLang desktop app corpus
why: Arcana-owned graphics/image boundary with backend-hosting responsibility for later higher graphics layers
current_source: rewrite-owned-in-progress
still_needs_rebuild: land reusable backend structure with `arcana_graphics.arcsb` as the first backend and leave room for later `iced_graphics` and Direct2D backends without collapsing the package to one backend forever
update_note: this layer should stay focused on graphics/image semantics and backend ownership and avoid turning into a retained-mode UI or scene framework by accident
promotion_condition: a rewrite-owned graphics grimoire exists, hosts the first backend path cleanly, and proves the desktop shell can hand off to graphics backends without reviving the old public canvas substrate

id: GRIMOIRE-TEXT
classification: bootstrap-required
role: text grimoire
current_package: `grimoires/libs/arcana-text`
historical_seed: archived MeadowLang desktop app corpus
why: Arcana-owned text draw, shaping, layout, and text-asset boundary above graphics/backing surfaces plus `std.text` and `arcana_process.fs`
current_source: rewrite-owned-in-progress
still_needs_rebuild: rebuild `arcana_text` into a full Arcana-owned paragraph/font/layout engine with shaping, fallback, and glyph rasterization while keeping file IO itself in `arcana_process.fs`
update_note: the scaffold-era `arcana_text -> arcana_desktop` label wrapper path has been removed from the public design center, and the provider-backed bootstrap path has been torn back out. `arcana_text` is again a plain first-party source library with the pinned Monaspace `v1.400` Variable asset set retained, while the old in-tree paragraph/provider implementation has been cleared so the engine can be rebuilt cleanly in Arcana source. Runtime no longer owns text-specific dispatch or fixed text opaque families, and the long-term direction remains the same: no desktop helper layer, no third-party text crate dependence, no public label-wrapper facade, and no runtime-builtin text grimoire.
promotion_condition: a rewrite-owned text grimoire exists and proves paragraph layout, text draw, and text-asset flows without pushing file APIs into grimoire policy or falling back to scaffold-era label wrappers

id: GRIMOIRE-AUDIO
classification: bootstrap-required
role: audio grimoire
current_package: `grimoires/libs/arcana-audio`
historical_seed: archived MeadowLang audio app corpus
why: Arcana-owned playback/audio boundary above the low-level `arcana_audio` substrate
current_source: rewrite-owned-in-progress
still_needs_rebuild: grow the current package into a miniaudio-class audio/device/playback layer once the runtime/backend audio seam is ready for real smoke execution
update_note: `arcana_audio` is now the public low-level audio owner above the WinAPI-backed helper substrate; keep ergonomic playback policy here instead of expanding the low-level layer into a full framework
promotion_condition: a rewrite-owned audio grimoire grows beyond bootstrap status on top of the active low-level `arcana_audio` lane and powers a basic audio smoke demo
