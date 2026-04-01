# Forewords Proof

This workspace is a small end-to-end forewords verification fixture.

It demonstrates:
- a public basic foreword exported from a dependency package
- a package-local foreword alias in the app package
- field, param, and function targets
- an executable metadata foreword backed by a checked-in toolchain product
- deterministic `#test` discovery

Workspace layout:
- `tool/`
  provides `tool.meta.trace` and `tool.exec.note`
- `app/`
  aliases `tool.meta.trace` as `app.meta.local` and applies the forewords

Verification commands:

```powershell
cargo run -p arcana-cli -- check examples/forewords-proof
cargo run -p arcana-cli -- foreword list examples/forewords-proof --format json
cargo run -p arcana-cli -- foreword show app.meta.local examples/forewords-proof --format json
cargo run -p arcana-cli -- foreword index examples/forewords-proof --format json
cargo run -p arcana-cli -- test --list examples/forewords-proof
```

What to look for:
- catalog entries for `tool.meta.trace`, `tool.exec.note`, and `app.meta.local`
- attached index entries for `app.meta.local` on:
  `app.Session.value`
  `app.helper(seed)`
  `app.helper`
- an attached index entry for `tool.exec.note` on `app.smoke`
- an emitted runtime-metadata entry for `tool.exec.runtime_note` on `app.smoke`
- a test listing line ending with `::app::smoke`

This example uses a checked-in Windows `.cmd` adapter product with a same-stem JSON sidecar.
