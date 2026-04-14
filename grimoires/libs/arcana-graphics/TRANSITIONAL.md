`arcana_graphics` is still transitional, but its direction is now backend-oriented rather than canvas-wrapper-oriented.

The first active backend is `arcana_graphics.arcsb`.

Nothing in the current surface should be treated as locking:

- the final higher-level Arcana graphics API
- the eventual `iced_graphics` port over this package
- the later Direct2D backend shape

What is locked for this phase:

- graphics backends live in this grimoire
- `arcana_graphics.arcsb` stays usable as a dependency surface for later grimoires
- canvas-era wrapper modules are retired rather than treated as the future graphics design
