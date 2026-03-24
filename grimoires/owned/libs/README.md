# Arcana-Owned App/Media Grimoires

This subtree holds rewrite-owned app/media grimoires.

Current grimoire lanes:
- `arcana-desktop`: authoritative desktop/window/input/events/run-loop package above the rewrite-owned substrate
- `arcana-graphics`: rewrite-owned 2D graphics/image package above `std.canvas`
- `arcana-text`: rewrite-owned text/layout package above `std.canvas`, `std.text`, and `std.fs`
- `arcana-audio`: rewrite-owned playback/audio package above `std.audio`
- `arcana-graphics`: graphics/image convenience above `std.canvas`
- `arcana-text`: text draw and text-asset convenience above `std.canvas`, `std.text`, and `std.fs`

Notes:
- File IO remains owned by `std.fs`.
- App/media grimoires may add asset-loading or rendering convenience on top of `std`, but they do not replace the underlying host-core substrate.
