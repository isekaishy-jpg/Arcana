`arcana_graphics` is transitional in the current rewrite phase.

This package is only being used to support `arcana_text` smoke and performance work.

The long-term CPU graphics design is deferred. Nothing in the current surface should be treated as locking:

- the eventual Arcana-owned graphics architecture
- the future `std` graphics/surface boundary
- any later binding or presentation strategy

For this phase, the only requirement is enough source-side graphics support to make `arcana_text` manually smokable and measurable.
