# Callable And Context Object Status

Status: `reserved-post-selfhost`

This document records the chosen direction for first-class callable capability without making it part of the selfhost baseline.

## Current Decision

- Arcana does not adopt closures in the selfhost baseline.
- Arcana does not adopt lambdas in the selfhost baseline.
- Arcana does not adopt general function values in the selfhost baseline.
- Arcana does not adopt dynamic-dispatch callable objects in the selfhost baseline.

## Chosen Future Direction

- If Arcana later gains first-class callable capability, the intended direction is explicit function objects plus explicit context objects.
- Context is meant to be explicit data, not implicit lexical capture.
- Callable behavior should be signature-visible and statically understandable.

## Why This Is Settled Now

- Page rollups and other structured cleanup features should not quietly depend on closures.
- Typed frontend and IR work should not invent placeholder closure semantics.
- Future callable support should solve the lack of closures through explicit objects, not by reopening closure semantics through the back door.

## Pre-Selfhost Guidance

- Parser, HIR, IR, std, and runtime work must not assume callable/context objects exist yet.
- New language work must not introduce hidden closure-like capture behavior.
- Cleanup handlers, callbacks, and similar surfaces remain named callable paths only until a dedicated callable-object contract exists.

## Required Future Contract Before Implementation

Implementation of function/context objects requires a dedicated scope doc that settles:

- type identity and generic behavior
- explicit context construction rules
- invocation syntax and lowering
- ownership, send/share, and async interaction
- trait/impl interaction
- host-boundary and reflection restrictions
