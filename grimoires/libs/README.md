# Arcana-Owned App/Media Grimoires

This subtree holds rewrite-owned app/media grimoires.

Current grimoire lanes:
- `arcana-desktop`: authoritative desktop/window/input/events/run-loop package above the rewrite-owned substrate
- `arcana-graphics`: rewrite-owned graphics/image package with backend-hosting responsibility
- `arcana-text`: rewrite-owned text/layout package above graphics/backing surfaces, `std.text`, and `arcana_process.fs`
- `arcana-audio`: rewrite-owned low-level playback/audio package and public audio owner

Notes:
- File IO remains owned by `arcana_process.fs`.
- App/media grimoires may add asset-loading or rendering convenience on top of `std`, but they do not replace core text/value substrate or binding-owned host backends.
