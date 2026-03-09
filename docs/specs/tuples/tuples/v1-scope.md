# Tuple v1 Scope

Status: `approved-pre-selfhost`

Tuples are part of the selfhost-facing language contract and must be explicit before typed frontend work hardens.

## Baseline Contract

- Arcana v1 supports pair tuples only.
- Tuple type syntax is `(A, B)`.
- Tuple literal syntax is `(a, b)`.
- Nested pairs are allowed.
- Three-or-more-element tuples are not part of the contract.

## Access Contract

- Tuple field access is positional only.
- Only `.0` and `.1` are valid tuple field selectors.
- `.2` and above are invalid in v1.
- Tuple access remains distinct from record field access even though both use `.` syntax.

## Construction and Use

- Tuples are value aggregates.
- Whole-tuple construction and whole-value passing/returning are supported.
- Tuples may appear in `match` and equality where element types permit it.
- Tuples may be nested to build protocol payloads, but named records are preferred once the shape becomes semantically meaningful.

## Explicit Exclusions

- No tuple destructuring in bindings, parameters, or `for` headers.
- No named tuple fields.
- No tuple methods or special tuple traits.
- No variadic tuple families.
- No tuple field assignment such as `pair.0 = x`.

## Equality and Type Behavior

- Pair equality is structural and order-sensitive.
- `(A, B)` and `(B, A)` are different types unless `A` and `B` happen to be the same type.
- Copy/send/share behavior is component-wise and follows ordinary Arcana type rules.
- Layout is not a public source-visible ABI contract.

## Guidance

- Use tuples for small transient multi-value returns and protocol rows.
- Prefer named records when repeated positional access starts carrying domain meaning.
- The existing anonymous-shape positional-access lint direction remains valid and should stay part of diagnostics work.
