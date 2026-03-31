# Cleanup Footer Replacement Design

Status: `reference-only`

This note records the design rationale behind the approved cleanup-footer replacement.
The active authority is:

- [v1-scope.md](/c:/Users/Weaver/Documents/GitHub/Arcana/docs/specs/page-rollups/page-rollups/v1-scope.md)
- [deferred-roadmap.md](/c:/Users/Weaver/Documents/GitHub/Arcana/docs/specs/page-rollups/page-rollups/deferred-roadmap.md)

## Landed Direction

The old page-rollup cleanup form:

```arcana
[x, h]#cleanup
```

is replaced by cleanup footers:

```arcana
-cleanup
-cleanup[target = value]
-cleanup[target = value, handler = path.to.cleanup]
```

Key landed decisions:

- cleanup remains a footer in its current approved form
- bare `-cleanup` means whole owning scope cleanup, not single-target cleanup
- `-cleanup` is valid only in attached post-owner footer position
- this patch does not define a generic `-name` footer family
- headed regions are separate inner structural blocks, not attached footer forms
- future headed-region or other dash-form work may introduce other `-name` uses in other contexts without rewriting cleanup footer semantics

## Compatibility Constraint For Future Headed Regions

This cleanup-footer replacement intentionally does not claim that all future `-name` forms are footers.

That leaves room for later features to use `-cleanup` or other dash names as:

- region rides
- modifiers
- overrides
- other non-footer structural forms

without invalidating the narrower cleanup-footer contract.

## Why The Old Surface Was Replaced

- `#cleanup` looked like an ad hoc postfix contract instead of a first-class cleanup declaration
- bare whole-scope cleanup needed a direct surface
- loop cleanup semantics needed to move to body-scope exit rather than one final loop epilogue
- the feature needed to become first-class through syntax, HIR, frontend, IR, runtime, and native lowering rather than remaining a partly special-cased metadata lane

## Implementation Notes

- historical internal names may still mention `page rollup` during migration
- user-facing diagnostics, tests, and approved docs should prefer `cleanup footer`
- future headed-region work should reuse cleanup machinery where useful, but should not assume cleanup footers are transitional or obsolete
