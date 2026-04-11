# Arcana Text Smoke/Perf Pass With No Rust or Runtime Changes

## Summary
- Do not touch `crates/*`, runtime host code, CABI, OS bindings, or `std.canvas` architecture in this phase.
- Treat `arcana_graphics` as transitional for now and add an in-grimoire note that its current surface is temporary and exists only to support proof/smoke work until the later graphics pass.
- Focus this phase entirely on `arcana_text` smokeability and performance using Arcana-source-only changes.

## Implementation Changes
- Make `--ui-demo` the real manual smoke window again.
  - Remove the host-label-only fallback demo as the default manual path.
  - Use the existing Arcana text render path and the existing source-side glyph painting path already present in the proof app.
  - Do not depend on new image-upload, softbuffer, or runtime features.
- Keep presentation source-only and conservative.
  - Use the current row-run / rect-based glyph paint path as the authoritative on-screen proof path for this phase.
  - Do not add any new graphics architecture or attempt to rehabilitate `std.canvas`.
  - If any existing unused image helper remains in the proof, leave it non-authoritative unless it already works with zero Rust changes.
- Add caching so manual smoke is usable.
  - Prebuild the current proof draw streams for the active stress preset.
  - Rebuild only when the stress preset changes, window-dependent layout changes, or proof configuration changes.
  - On normal redraws, paint cached streams and overlay only; do not reshape/raster every frame unless the preset changed.
- Add a simple in-window stress control.
  - Add one clickable button in the proof window that cycles fixed presets: `Normal`, `Dense`, `Heavy`.
  - `Normal`: current proof layout once.
  - `Dense`: current proof layout plus one repeated stress block using the same Arcana text path.
  - `Heavy`: larger repeated stress block intended to visibly lower FPS.
  - Stress content must stay real Arcana text rendering, not host labels.
- Add live FPS and frame timing overlay.
  - Show FPS in the window.
  - Update the displayed FPS at 4 Hz only using `std.time.monotonic_now_ms`.
  - Drive demo-mode redraws continuously so FPS is meaningful.
  - Overlay `FPS`, `last paint ms`, and current stress preset.
- Add headless perf reporting to the existing smoke flows.
  - Keep `--smoke`, `--smoke-features`, and `--smoke-axis`.
  - Extend the scratch report to include timings for the proof-local text phases that already exist in source:
    - snapshot / shape-layout build
    - draw-stream / raster build
    - total render time per sample
    - total smoke time
  - Do not add deep internal profiler plumbing in Rust.
- Add the transitional graphics note.
  - Put a short repo-local note under `grimoires/libs/arcana-graphics` stating that:
    - the current grimoire is transitional
    - the long-term CPU graphics design is deferred
    - this phase does not lock the future graphics/std/binding architecture
    - current graphics use is only enough to support `arcana_text` smoke and perf work

## Test Plan
- Headless:
  - `--smoke-features` still reports distinct feature-off/feature-on glyph and pixel results.
  - `--smoke-axis` still reports nonzero axis-driven output.
  - Smoke report file now includes timing fields for each render sample and total run time.
- Manual:
  - `--ui-demo` opens a responsive window showing real Arcana-rendered text.
  - The stress button cycles `Normal` -> `Dense` -> `Heavy` and requests a rebuild/redraw.
  - FPS overlay updates at roughly 250 ms intervals rather than every frame.
  - Higher stress presets visibly reduce FPS.
- Regression:
  - No changes under `crates/*`.
  - No runtime-host or Rust-side graphics changes.
  - No `std.canvas` scope change in this phase.

## Assumptions
- Manual FPS in this phase is a transitional end-to-end metric that includes the current Arcana-source paint path; it is not the final graphics-stack benchmark.
- Headless smoke timings are the authoritative engine-side performance signal for `arcana_text`.
- The proof app remains the only place where manual stress UI is added in this phase; `arcana_graphics` is not expanded beyond documentation and any strictly unavoidable proof support already possible in Arcana source.
