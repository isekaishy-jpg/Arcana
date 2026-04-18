# Callable Status

Status: `approved-pre-selfhost`

This document records the current pre-selfhost callable contract after callable struct values were approved.

## Current Decision

- Arcana does not adopt closures in the selfhost baseline.
- Arcana does not adopt lambdas in the selfhost baseline.
- Arcana does not adopt general function values in the selfhost baseline.
- Arcana does not adopt dynamic-dispatch callable objects in the selfhost baseline.
- Arcana does adopt explicit callable struct values in the selfhost baseline.
- Arcana does adopt explicit `obj` and `create ... scope-exit` support pre-selfhost; callable/context object roles inside that approved object model remain separate from callable struct dispatch.

## Approved Callable Struct Contract

- Callable struct value dispatch is `:: call` only.
- Callable struct dispatch is explicit trait-contract dispatch, not name-based magic.
- Approved callable contract families are:
  - `CallableRead0[Out]`
  - `CallableEdit0[Out]`
  - `CallableTake0[Out]`
  - `CallableRead[Args, Out]`
  - `CallableEdit[Args, Out]`
  - `CallableTake[Args, Out]`
- The matching lang items are:
  - `call_contract_read0`
  - `call_contract_edit0`
  - `call_contract_take0`
  - `call_contract_read`
  - `call_contract_edit`
  - `call_contract_take`
- Receiver modes for callable structs are `read`, `edit`, or `take`.
- `hold self` is not part of the callable contract in v1.
- Callable-contract traits and callable-contract impls that use `hold self` are rejected at declaration time.
- Packed callable args are always `take args: Args`.
- Callable-value dispatch is struct-only in this phase:
  - `record`, `union`, `array`, and `obj` do not participate in callable-value dispatch.
- `RecordType :: named_fields :: call` remains constructor sugar only; record values are not callable.
- Record/struct constructor sugar may omit `Option[T]` fields, and `Type :: :: call` is valid when no required fields remain.

## Why This Is Settled Now

- Cleanup footers and other structured cleanup features should not quietly depend on closures.
- Typed frontend and IR work should not invent placeholder closure semantics.
- The absence of closures now has an approved static replacement for value-call transport: callable structs.
- Callable structs solve the closure gap without reopening implicit capture or dynamic callable transport.

## Phrase Arity Interaction

- The 3-top-level-arg cap on qualified and memory phrases is intentional.
- Callable struct dispatch uses explicit packing:
  - `f :: a :: call` uses `Args = A`
  - `f :: a, b :: call` uses `Args = (A, B)`
  - `f :: a, b, c :: call` uses `Args = (A, B, C)`
- Higher-arity data should stay explicit through ordinary records, tuples, or statement-form attached metadata where that surface fits.

## Pre-Selfhost Guidance

- Parser, HIR, IR, std, and runtime work may assume callable structs exist as approved v1 surface.
- Parser, HIR, IR, std, and runtime work must not assume any broader callable/context-object transport contract beyond the current approved object/owner model and callable-struct surface.
- New language work must not introduce hidden closure-like capture behavior.
- Cleanup handlers, callbacks, and similar path-only surfaces remain named callable paths only in this phase.
- Parser/HIR/IR work must not treat the 3-arg phrase cap as justification for early callable/context-object design or hidden closure transport.

## Reserved Future Area

- If Arcana later gains broader first-class callable transport beyond callable structs and the current object/owner model, the intended direction is still explicit function objects plus explicit context objects.
- Context is meant to be explicit data, not implicit lexical capture.
- Any future callable-object expansion still requires a dedicated contract that settles:
  - type identity and generic behavior
  - explicit context construction rules
  - invocation syntax and lowering
  - ownership, send/share, and async interaction
  - trait/impl interaction
  - host-boundary and reflection restrictions
