# Cleanup Footer Replacement Design

Status: `reference-only`

## Purpose

This document captures a replacement design direction for the current page-rollup cleanup surface.

It is not the active contract.
Current authority remains:

- `docs/arcana-v0.md`
- `docs/specs/spec-status.md`
- `docs/specs/page-rollups/page-rollups/v1-scope.md`
- `docs/specs/page-rollups/page-rollups/deferred-roadmap.md`

This note exists because the current page-rollup model has drifted away from the intended shape:

- cleanup should be a footer, not a foreword-like form
- cleanup should read as a scope-exit declaration, not as delayed qualified-phrase sugar
- the bare cleanup form should mean "defer cleanup for the owning block"
- explicit target and handler should be optional refinements, not the whole feature

## Design Summary

Replace the current `#cleanup` page-rollup surface with a footer-form scope-exit declaration:

```arc
-cleanup
-cleanup[target = value]
-cleanup[target = value, handler = path.to.cleanup]
```

This proposal treats `-cleanup` as the first member of a broader scope-exit footer family.

Prefix forms and footer forms stay separate:

- `#...` remains foreword / metadata syntax
- `-...` becomes footer / scope-exit syntax

That separation is intentional and should remain strict.

## Why Replace The Current Model

The current page-rollup spec defines cleanup as:

```arc
[x, h]#cleanup
```

which semantically lowers as:

```arc
h :: x :: call
```

That is workable, but it models cleanup primarily as an owner-exit scheduled call.
It does not read like a true footer-form cleanup declaration, and it keeps the surface too close to qualified phrase syntax.

The replacement direction makes cleanup a first-class scope-exit feature instead:

- footer-only
- distinct from forewords
- block-owned by default
- optionally narrowed by target
- optionally overridden by explicit handler

## Footer Family Model

Arcana should reserve `-name` forms for scope-exit footer declarations.

Conceptual split:

- prefix metadata:
  - `#test`
  - `#boundary[...]`
  - other forewords
- footer scope-exit declarations:
  - `-cleanup`
  - future siblings in the same family

Rule of thumb:

- `#...` modifies what follows
- `-...` finalizes what just ended

This proposal only defines `-cleanup`, but it intentionally leaves room for later footer-family work.

## Core Rule

A block-owning owner may be followed by zero or more footer declarations after its body dedents back to the owner indentation.

In this proposal, only one cleanup footer is defined:

- `-cleanup`

If no footer is present, behavior is unchanged from today.

## Syntax

Canonical v1 replacement forms:

```text
-cleanup
-cleanup[target = name]
-cleanup[target = name, handler = path]
```

Rules:

- footer payloads use named fields only
- positional payloads are not allowed
- `handler` without `target` is not allowed in the replacement v1
- `target` must be a binding name
- `handler` must be a statically resolved callable path

Examples:

```arc
fn run() -> Int:
    let file = open_file :: :: call
    return 0
-cleanup
```

```arc
fn run() -> Int:
    let file = open_file :: :: call
    return 0
-cleanup[target = file]
```

```arc
fn run() -> Int:
    let file = open_file :: :: call
    return 0
-cleanup[target = file, handler = std.fs.close]
```

## Placement And Attachment

`-cleanup` is footer-only.
It is not a foreword, not an expression, and not a statement inside the owner body.

Attachment rules:

- a cleanup footer must appear immediately after the owning body dedents to owner indentation
- it attaches to the immediately preceding completed owner
- it never attaches to an inner nested block by accident
- if a normal sibling statement or item appears first, attachment is lost and the footer is invalid

This keeps footer syntax distinct from forewords:

- forewords are prefix metadata above a target
- cleanup is footer policy below a completed owner

## Valid Owners

Valid owners for the replacement cleanup footer:

- `fn`
- `async fn`
- `behavior`
- `system`
- `if ... else ...` as one full construct
- `while`
- `for`
- statement-form qualified phrases with attached blocks
- statement-form memory phrases with attached blocks

These match the current page-rollup ownership slice, but the semantics below change for cleanup itself.

## Conceptual Model

`-cleanup` is a footer-form scope-exit declaration for the owning block scope.

It is not defined as "call handler with subject later."
That call may be one implementation consequence, but it is not the language-level definition.

The language-level definition is:

- the owner has an explicit cleanup policy
- cleanup-covered bindings become active as the owner executes
- when the owner scope exits, the cleanup policy runs

## Covered Binding Set

### Bare Footer

```arc
-cleanup
```

Meaning:

- apply cleanup to every active cleanup-capable binding in the owning scope

For routines:

- parameters of the routine are part of the owning scope and may be covered if cleanup-capable

For block-owning statements:

- only bindings introduced in that owner's block scope are covered

### Targeted Footer

```arc
-cleanup[target = name]
```

Meaning:

- apply cleanup only to `name`

### Target + Handler Override

```arc
-cleanup[target = name, handler = path]
```

Meaning:

- apply cleanup only to `name`
- use `path` instead of the default cleanup contract for that binding

## Cleanup-Capable Bindings

A binding is cleanup-capable only if its static type has an explicit cleanup contract.

This proposal intentionally requires that contract to be:

- explicit
- statically resolved
- auditable
- free of hidden fallback rules

This proposal does not freeze the exact contract carrier yet.
The authoritative replacement scope would need to choose one concrete mechanism, such as:

- a dedicated lang-item-backed cleanup contract
- a dedicated trait-based cleanup contract
- an owner/type contract with the same explicit static properties

What is not allowed:

- convention-based magic method lookup
- implicit destructor discovery
- dynamic callable lookup
- closure-based cleanup dispatch

The important design rule is that bare `-cleanup` must not depend on hidden cleanup inference.

## Activation Model

Cleanup coverage is activation-based.

Rules:

- a parameter becomes active at owner entry
- a local becomes active only after successful initialization
- if initialization never completes, cleanup does not run for that binding
- a targeted binding must resolve in the owning scope, not in a nested child-only scope

## Exit Model

Cleanup runs whenever the owning scope exits.

Covered exits:

- normal fallthrough
- `return`
- `break`
- `continue`
- `?` propagation that exits the owner

Loop note:

- this replacement design treats cleanup as attached to the owner block scope, not as a one-shot loop-statement epilogue
- for loop owners, that means bindings declared in the loop body clean up when the loop body scope exits
- this is an intentional shift away from the current page-rollup v1 loop-exit-only cleanup rule

## Execution Order

Execution order must be explicit:

1. inner nested scope cleanup runs before outer-owner cleanup
2. ordinary local `defer` work in the owner runs before the owner's `-cleanup`
3. cleanup work runs in reverse lexical activation order among the bindings actually covered

Targeted cleanup does not change the global ordering rule.
It narrows the covered set, then applies the same ordering model.

## Static Restrictions

To keep the feature easy to reason about:

- a cleanup-covered binding may not be moved after activation
- a cleanup-covered binding may not be reassigned after activation
- there is no disarm or transfer operation in replacement v1
- cleanup handler failure is fail-fast unless a later authoritative scope says otherwise

These rules apply both to bare `-cleanup` coverage and to targeted cleanup.

## Handler Rules

When an explicit handler is present:

- it must resolve statically to a named callable path
- it must be synchronous in replacement v1
- it must accept exactly one parameter
- its parameter type must be compatible with the target binding type

Bare cleanup does not implicitly manufacture a handler name.
It uses the binding's default cleanup contract only.

## Diagnostics

The replacement design should provide deterministic diagnostics for:

- footer without a valid owning block
- malformed footer payload
- unknown footer field
- `handler` without `target`
- non-binding `target`
- target not available in the owning scope
- target not cleanup-capable
- unresolved or non-callable handler
- async handler where sync cleanup is required
- move or reassignment after cleanup activation

## Parser Separation From Forewords

The parser should enforce a strict separation:

- `#...` is parsed only as a foreword in prefix metadata position
- `-cleanup...` is parsed only as a footer declaration in dedented post-owner position

This proposal intentionally avoids shared syntax between the two systems.

That avoids confusion such as:

- treating cleanup as a kind of attribute
- visually mixing owner-exit policy with prefix metadata
- future ambiguity when the footer family grows

## Migration From Current Page Rollups

If Arcana adopts this replacement direction:

- `[x, h]#cleanup` would migrate to `-cleanup[target = x, handler = h]`
- the bare `-cleanup` form would become new surface with no exact predecessor
- current user-facing "page rollup" terminology should be retired in favor of cleanup footer / scope-exit footer language
- "page rollup" may remain as an internal historical or migration label only

## Explicit Non-Goals

This replacement v1 does not require:

- global RAII
- hidden destructor semantics
- handler inference by naming convention
- handler-only blanket cleanup overrides
- general footer payload expressions
- callable values or closures as cleanup handlers

## Open Follow-Up Required For An Authoritative Replacement

An active replacement scope would still need to settle:

- the exact static cleanup contract carrier
- whether multiple footer declarations may stack on one owner
- whether footer-family siblings should share common payload conventions
- whether loop cleanup should expose any separate statement-exit form distinct from body-scope exit
- how cleanup footers interact with future broader scope-exit footer features

## Replacement Direction

The important replacement direction is:

- cleanup is a footer declaration
- cleanup belongs to a future footer family
- bare cleanup means block-owned cleanup policy
- target and handler are optional refinements
- no hidden rules are allowed

That is a substantially different model from the current page-rollup definition and should be treated as a real design replacement, not as a syntax rename.
