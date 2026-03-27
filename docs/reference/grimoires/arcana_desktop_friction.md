# `arcana_desktop` Bring-Up Friction

This note is reference-only. Authority for the desktop surface still lives in approved specs and crate behavior.

## Purpose

Record the real friction found while bringing `arcana_desktop` up to the current Windows-first parity bar, including the large native proof workspace and the real `app.exe + arcwin.dll` validation lane.

The large proof now lives as a normal checked-in workspace at `examples/arcana-desktop-proof`.
It is packaged through the standard member/target flow:
- `app` as `windows-exe`
- `arcwin.dll` as a staged child product selected by `native_child = "default"` on the desktop dependency

The CLI integration proof packages that exact workspace instead of generating a special test-only source tree.

## Resolved Friction

### 1. Mixed-workspace native packaging over-targeted unrelated members

Problem:
- Packaging a selected member for `windows-exe` or `windows-dll` originally planned the requested target across the full workspace order.
- In mixed workspaces, that made unrelated members and dependency libraries enter the wrong native lane.
- The most visible failure was `windows-dll` packaging of a selected library in a workspace that also contained app members and library dependencies.

Resolution:
- Package-mode planning is now root-aware.
- Only the selected package member receives the requested native target.
- Workspace dependencies of that selected member build as `internal-aot`.
- Unrelated workspace members are not forced into the requested package target.

Implication for future grimoires:
- Multi-member workspaces are now viable for large native desktop proof apps and supporting libraries without requiring “single-member workspace” test shaping.

### 2. Lockfile format assumed every member already had build entries

Problem:
- `Arcana.lock` v2 originally required every workspace member to have at least one `[builds."<member>"]` target entry.
- That did not fit package-mode partial builds, where only the selected member closure is intentionally built.

Resolution:
- Lockfile rendering and reading now allow members with empty target sets.
- Unbuilt members remain present in workspace metadata without synthetic artifact entries.

Implication for future grimoires:
- Partial native packaging flows no longer need fake builds just to keep lockfiles valid.

### 3. Native DLL export collection was too loose for linked package artifacts

Problem:
- The native DLL lane originally derived exports from linked routine metadata broadly enough to trip over dependency exports, including generic std routines.
- In practice, this surfaced as `windows-dll target does not support generic export ...` failures when building desktop-dependent libraries.

Resolution:
- Native DLL export collection now follows the root package exported surface rows for real package artifacts.
- Synthetic tests without surface metadata still fall back to the older root-module-prefix rule.

Implication for future grimoires:
- Desktop-dependent libraries can link against large dependency graphs without leaking dependency exports into the native DLL ABI.

### 4. Arcana desktop package boundaries are not the same thing as the native DLL runtime boundary

Problem:
- Treating the source-level `arcana_desktop` grimoire itself as the sibling runtime DLL looked attractive at first, but it is the wrong boundary.
- The grimoire API is generic and callback-driven (`Application[A]`), which means a standalone sibling DLL at that source/package layer would require a new callback ABI between the app exe and the DLL.
- Rich desktop records such as `WindowConfig`, `CursorSettings`, and text-input records are valid Arcana package APIs, but they are not the same thing as the narrow native DLL ABI boundary.

Resolution:
- The real sibling DLL boundary was moved under the grimoire, to the native runtime/provider layer.
- Apps still compile their Arcana-facing `arcana_desktop` source usage normally, but native bundles can now stage `arcwin.dll` as a declared child product chosen through dependency metadata.

Implication for future grimoires:
- There is a hard distinction between:
  - in-process Arcana package APIs
  - staged runtime-provider DLLs
  - native `windows-dll` ABI exports for source-level Arcana libraries
- Desktop/window/input/text records are appropriate for Arcana package boundaries, not for the narrow native ABI boundary used by `windows-dll`.

### 5. Runtime-DLL bundles originally depended on a Rust `dylib` closure

Problem:
- Once `native_delivery = "dll"` moved the desktop runtime boundary into a real sibling `arcwin.dll`, the initial implementation staged a Rust `dylib` provider.
- On Windows that also dragged in a toolchain `std-*.dll` closure, which was the wrong long-term packaging shape.

Resolution:
- The desktop sibling DLL is now built as a real declared `cdylib` child product from manifest metadata.
- Bundle staging resolves declared child products directly and no longer scavenges Rust toolchain `std-*.dll` files.

Implication for future grimoires:
- Dependency-scoped native products now have an honest declared bundle shape.
- Future child/plugin products must keep their non-system closure explicit through declared sidecars, not through toolchain scavenging.

## Current Practical Guidance

When building native desktop libraries:
- Keep rich desktop types inside Arcana package APIs.
- Export only ABI-supported signatures from `windows-dll` roots.
- Treat helper-module `export` as part of the library’s public/native surface, not as a harmless internal convenience.

When building large desktop workspaces:
- Prefer a root app member for native `.exe` proof.
- Select `native_child = "default"` on the desktop dependency when you want a sibling runtime DLL instead of baking the desktop runtime seam into the bundle executable.
- Treat `windows-dll` as a separate source-library ABI lane, not as the desktop runtime-provider lane.

## Remaining Friction

### 1. Native IME update-payload automation is still not fully closed for bundle smoke

Current state:
- Native bundle proof exists for committed text and IME start/cancel lifecycle on the real Win32 path.
- In-process native host proof exists for committed IME composition payload delivery.
- Deterministic native bundle automation for live IME composition-update payloads is still open.

Why it matters:
- This is a testing/automation gap, not evidence that the desktop/text-input substrate is fake.
- It should remain explicit until a stable cross-process automation harness exists.
