# Split `record` From `struct` And Add Packed Callable Structs

## Summary
- Make `record` and `struct` genuinely separate language families instead of letting `struct` ride on record-shaped carriers.
- Keep `record` as semantic named data with its own shaping/constructor lane.
- Make `struct` the layout-bearing nominal type that owns generic construction, bitfields, and first-class callable values.
- Keep `array` in its own lane: constructor-callable and array-region-capable, but not callable as a value.
- Callable structs are added only to `:: call` value dispatch. Existing named-path-only callable surfaces remain unchanged.

## Key Changes
- `record` stays:
  - semantic named data
  - ordinary trait/method-capable nominal type
  - `RecordType :: named_fields :: call` as constructor sugar only
  - `record yield` / `record deliver` / `record place`
  - semantic `from <base>` copy-and-refine behavior
- `record` stops owning:
  - callable-value dispatch
  - generic `construct`
  - bitfields
  - layout/ABI-facing identity
- `struct` owns:
  - `StructType :: named_fields :: call`
  - `struct yield` / `struct deliver` / `struct place`
  - exact-same-type `from <base>`
  - bitfields
  - generic `construct`
  - callable-value dispatch
- `array` stays unchanged:
  - constructor-callable
  - `array yield` / `array deliver` / `array place`
  - never participates in callable-value dispatch
- `construct` targets `struct` and enum payload variants only; it rejects `record`.

- Replace shared record machinery with distinct syntax/HIR/IR/runtime carriers and foreword targets for `Record`, `Struct`, and `Union`. `Array` stays on its own existing carrier family. No shared record carrier remains responsible for `struct` or `union`.

- Bundle 3-tuples and use packed callable args:
  - `f :: :: call` uses a zero-arg callable contract
  - `f :: a :: call` uses `Args = A`
  - `f :: a, b :: call` uses `Args = (A, B)`
  - `f :: a, b, c :: call` uses `Args = (A, B, C)`
  - top-level call arg count stays capped at 3

- Use separate callable trait families per receiver mode:
  - `CallableRead0[Out]`: `fn call(read self: Self) -> Out`
  - `CallableEdit0[Out]`: `fn call(edit self: Self) -> Out`
  - `CallableTake0[Out]`: `fn call(take self: Self) -> Out`
  - `CallableRead[Args, Out]`: `fn call(read self: Self, take args: Args) -> Out`
  - `CallableEdit[Args, Out]`: `fn call(edit self: Self, take args: Args) -> Out`
  - `CallableTake[Args, Out]`: `fn call(take self: Self, take args: Args) -> Out`
- Add matching lang items for each callable contract family.
- `hold self` has no callable contract in this wave.
- `record`, `union`, `array`, and `obj` cannot implement callable contracts in this wave.

- Pin `:: call` resolution order:
  1. builtin numeric conversion subjects
  2. named callable paths and existing callable symbol resolution
  3. nominal constructor subjects for `record`, `struct`, `array`, and enum payload variants
  4. otherwise, struct-value callable-contract dispatch
- Bare-method lookup, dotted callable qualifiers, cleanup handlers, callbacks, and similar path-only surfaces remain unchanged.

- Pin receiver/access law for callable structs:
  - `read self` may use any subject expression
  - `edit self` requires a mutable local place subject, following existing `edit` param law
  - `take self` follows ordinary take/move law
  - packed args are always `take args: Args`
  - borrowed/retained access only comes from explicit capability values at the call site
  - callable dispatch never auto-borrows, auto-edits, auto-holds, or synthesizes `&...`

- Pin retained record constructor sugar:
  - `RecordType :: named_fields :: call` is named-field constructor sugar only
  - no attached blocks
  - unknown fields reject
  - duplicate fields reject
  - required fields must be present
  - optional fields may be omitted
  - this lowers through a record-specific constructor lane, not callable-trait dispatch and not generic `construct`

- Update approved docs in the same wave:
  - headed-regions scope must explicitly approve distinct `record`, `struct`, `union`, and existing `array` families
  - callable docs must make callable structs the approved closure-gap answer
  - access-mode wording must state that callable receivers are `read` / `edit` / `take` only, with packed args always `take`
  - path-only callable surfaces must stay path-only
  - `llm.md` must match the new split

## Test Plan
- Parser/HIR:
  - `record`, `struct`, and `union` regions lower into distinct carriers
  - `array` region behavior remains unchanged
  - 3-tuple syntax parses and lowers
  - callable trait families and `call[(A, B), Out]` / `call[(A, B, C), Out]` parse correctly
- Frontend:
  - `construct` accepts `struct` and enum payload variants, rejects `record`
  - `record` constructor sugar remains legal under the pinned constructor-only law
  - `record` may use ordinary traits/methods but cannot implement callable contracts
  - `struct ... from <base>` requires exact same nominal struct type
  - `record ... from <base>` keeps semantic copy-and-refine behavior
  - `hold self` callable contracts are rejected
  - `edit self` callable dispatch rejects non-place and immutable subjects
- Call dispatch:
  - `CallableRead*`, `CallableEdit*`, and `CallableTake*` each work through the matching receiver mode
  - packed args are consumed via `take args`
  - move-only inputs work through callable struct dispatch
  - explicit `&read` / `&edit` / `&take` / `&hold` args work only when spelled explicitly
  - constructor subjects beat struct-value callable dispatch
  - multiple matching callable contracts produce an ambiguity diagnostic
- IR/runtime:
  - separate execution lanes exist for record regions, struct regions, struct constructor calls, and struct-value callable dispatch
  - `array` execution remains separate and unchanged
  - no shared record exec carrier remains responsible for struct or union behavior
- Migration:
  - first-party record constructor call sites remain valid
  - first-party record-region uses remain valid
  - struct/union lowering no longer routes through record carriers
  - path-only callable surfaces continue rejecting callable-value inputs

## Assumptions
- This is a hard cut for the internal/public split, but not for record constructor sugar.
- `record` remains a first-class nominal type with ordinary impl/trait behavior.
- Callable-value dispatch is struct-only in this wave.
- `array` remains in its own existing lane.
- `hold self` is deferred.
- Larger callable input sets must be explicitly packaged; no variadics or implicit arg bundling beyond 3-tuples.
