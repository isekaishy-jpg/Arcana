# Tuple v1 Scope

Status: `approved-pre-selfhost`

Tuples are part of the selfhost-facing language contract and must be explicit before typed frontend work hardens.
The current stabilization point is explicit 2- and 3-tuples, not unbounded tuple families.

## Baseline Contract

- Arcana v1 supports 2- and 3-tuples only.
- Tuple type syntax is:
  - `(A, B)`
  - `(A, B, C)`
- Tuple literal syntax is:
  - `(a, b)`
  - `(a, b, c)`
- Nested tuples are allowed.
- Exact recursive tuple destructuring is supported in `let` bindings and `for` headers.
- Four-or-more-element tuples are not part of the contract.
- This is the current selfhost baseline, not a claim that generalized tuples are undesirable.

## Access Contract

- Tuple field access is positional only.
- Only `.0`, `.1`, and `.2` are valid tuple field selectors in v1.
- `.3` and above are invalid in v1.
- Tuple access remains distinct from record field access even though both use `.` syntax.

## Construction and Use

- Tuples are value aggregates.
- Whole-tuple construction and whole-value passing/returning are supported.
- Tuple destructuring is exact-shape only in current v1:
  - `let (left, right) = pair`
  - `let (first, second, third) = triple`
  - `for (left, right) in values:`
  - `for (first, second, third) in values:`
- Nested exact tuple destructuring is allowed inside those `let` and `for` forms.
- Tuple-specific `match` patterns are not part of the v1 contract.
- Current tuple use around `match` is limited to ordinary whole-value flow plus explicit positional access before matching.
- Tuples may be nested to build protocol payloads, but named records are preferred once the shape becomes semantically meaningful.

## Explicit Exclusions

- No tuple destructuring in parameter lists.
- No tuple patterns in `match`.
- No named tuple fields.
- No tuple methods or special tuple traits.
- No variadic tuple families.
- No tuple field assignment such as `pair.0 = x`.

## Narrowing Note

- Older frozen summary text used broader tuple/match wording.
- The rewrite v1 baseline intentionally does not carry Meadow-era tuple-pattern behavior forward as implicit contract.
- If tuple patterns return, they must come back through an explicit redesign and updated coverage, not by inference from archived behavior.

## Equality and Type Behavior

- Supported tuple equality is structural and order-sensitive.
- `(A, B)` and `(B, A)` are different types unless `A` and `B` happen to be the same type.
- `(A, B, C)` is distinct from every other tuple shape with different arity or ordering.
- Copy/send/share behavior is component-wise and follows ordinary Arcana type rules.
- Layout is not a public source-visible ABI contract.

## Guidance

- Use tuples for small transient multi-value returns, protocol rows, and packed callable-struct args.
- Prefer named records when repeated positional access starts carrying domain meaning.
- The existing anonymous-shape positional-access lint direction remains valid and should stay part of diagnostics work.

## Forward Path

- 2- and 3-tuples exist to keep the selfhost baseline tractable while typed ownership and layout rules are still settling; this is a staging constraint, not a philosophical rejection of richer tuple support.
- Generalized tuples remain the intended expansion path once the typed frontend, ownership rules, and selfhost baseline are stable enough to absorb them cleanly.
- If 2/3-tuple support becomes a demonstrated selfhost blocker, further tuple enrichment may be reconsidered only through an explicit freeze exception and updated conformance coverage.
- Deferred follow-up items are tracked in `docs/specs/tuples/tuples/deferred-roadmap.md`.
