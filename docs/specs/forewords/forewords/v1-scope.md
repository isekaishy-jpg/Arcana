# Forewords v1 Scope

## Implemented in Plan 30

### Syntax
- `#name`
- `#name[arg]`
- `#name[arg1, arg2]`
- `#name[key = value]`
- `#name[arg1, key = value]`

### Built-in compiler-owned forewords
- `#deprecated["message"]`
- `#only[os = "...", arch = "..."]`
- `#test`
- `#allow[...]`
- `#deny[...]`
- `#inline`
- `#cold`

### v1 targets
- top-level declarations
- `import`, `reexport`, `use`
- trait methods and impl methods

Not supported in v1:
- field-level/param-level targets
- statement-level/expression-level targets

### Comment cutover
- `#` comments are removed
- `//` is the line and inline comment form
- `#` is reserved for forewords

### Current lint-control coverage
`#allow/#deny` currently governs `deprecated_use` behavior at call sites.

### CLI support
- `arcana test --list <grimoire-dir>` lists discovered `#test` functions.

## Explicit exclusions in v1
- `#derive`
- user-defined forewords (`foreword ...`)
- runtime-retained metadata and introspection
- statement/expression targets

## Policy
Any deferred foreword item must be tracked in:
- `docs/specs/forewords/deferred-roadmap.md`
with:
- target plan
- trigger condition
- owner
- acceptance criteria

