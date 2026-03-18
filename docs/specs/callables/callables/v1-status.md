# Callable And Context Object Status

Status: `reserved-post-selfhost`

This document records what still remains reserved after the pre-selfhost object/owner contract landed in `docs/specs/objects/objects/v1-scope.md`.

## Current Decision

- Arcana does not adopt closures in the selfhost baseline.
- Arcana does not adopt lambdas in the selfhost baseline.
- Arcana does not adopt general function values in the selfhost baseline.
- Arcana does not adopt dynamic-dispatch callable objects in the selfhost baseline.
- Arcana now does adopt explicit `obj` and `create ... scope-exit` support pre-selfhost; callable objects and context objects are roles inside that approved object model, not future closure placeholders.

## Chosen Future Direction

- If Arcana later gains broader first-class callable transport beyond the current object/owner model, the intended direction is still explicit function objects plus explicit context objects.
- Context is meant to be explicit data, not implicit lexical capture.
- Callable behavior should be signature-visible and statically understandable.

## Why This Is Settled Now

- Page rollups and other structured cleanup features should not quietly depend on closures.
- Typed frontend and IR work should not invent placeholder closure semantics.
- Future callable support should solve the lack of closures through explicit objects, not by reopening closure semantics through the back door.

## Phrase Arity Interaction

- The 3-top-level-arg cap on qualified and memory phrases is intentional.
- That cap does not, by itself, create a need for function/context objects.
- Higher-arity data should stay explicit through ordinary records, pair nesting, or statement-form attached metadata where that surface fits.
- Function/context objects remain reserved for future first-class callable transport/callback needs, not as an escape hatch for phrase arity.

## Pre-Selfhost Guidance

- Parser, HIR, IR, std, and runtime work must not assume callable/context objects exist yet.
- New language work must not introduce hidden closure-like capture behavior.
- Cleanup handlers, callbacks, and similar surfaces remain named callable paths only until a dedicated callable-object contract exists.
- Parser/HIR/IR work must not treat the 3-arg phrase cap as justification for early callable/context-object design.

## Required Future Contract Before Implementation

Implementation of broader callable transport beyond the current object/owner scope requires a dedicated contract that settles:

- type identity and generic behavior
- explicit context construction rules
- invocation syntax and lowering
- ownership, send/share, and async interaction
- trait/impl interaction
- host-boundary and reflection restrictions
