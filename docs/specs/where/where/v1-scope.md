# Where Semantics v1 Scope

Status: `approved-pre-selfhost`

This scope freezes the rewrite-era contract for `where` semantics.

## Included Predicate Families

- ordinary trait bounds
- trait/impl `where` requirements that play the role of supertrait-style constraints
- projection equality
- outlives predicates

## Contract Rules

- `where` is semantic language law, not merely preserved source text.
- The rewrite must carry a structured predicate model sufficient to validate and lower the approved predicate families.
- Frontend/type-law work must enforce `where` predicates semantically rather than treating them as descriptive strings.
- The structured predicate model may live in rewrite type-law/frontend phases even if syntax/HIR continue to preserve the original `where` text for round-tripping.
- v1 requires semantic validation of predicate shape, declared lifetimes/types, associated-type references, and trait-where requirements needed by impls; it does not require a separate user-visible `where` AST format.

## Projection Equality

- Projection equality remains part of the frozen surface.
- Example shape:
  - `where Iterator[I], Iterator[I].Item = U`
- Projection equality is not optional documentation text; it is part of the type-law contract.

## Outlives Predicates

- Outlives predicates remain part of the frozen surface.
- Supported forms include:
  - `'a: 'b`
  - `T: 'a`
- v1 requires these predicates to parse and validate as structured predicates against declared lifetime/type parameters.

## Dispatch Model

- Trait dispatch remains static/monomorphized.
- This scope does not ratify trait objects or dynamic dispatch.
- This scope does not introduce new public `where` syntax beyond the frozen summary.
