# Arcana-Owned App/Media Grimoires

This subtree holds rewrite-owned app/media grimoire scaffolds.

Current scaffolds:
- `arcana-desktop`: desktop/window/input/events/run-loop facade above `std.window`, `std.input`, `std.events`, and `std.time`
- `arcana-audio`: higher-level playback facade above `std.audio`
- `arcana-graphics`: graphics/image convenience above `std.canvas`
- `arcana-text`: text draw and text-asset convenience above `std.canvas`, `std.text`, and `std.fs`

Notes:
- File IO remains owned by `std.fs`.
- App/media grimoires may add asset-loading or rendering convenience on top of `std`, but they do not replace the underlying host-core substrate.
