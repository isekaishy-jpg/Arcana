# Access Capability Convergence Before Raw CABI Expansion

## Summary
- Replace the split ownership model with one explicit capability model centered on four access modes: `read`, `edit`, `take`, and `hold`.
- Replace the old reference surface with explicit capability syntax:
  - expression forms: `&read x`, `&edit x`, `&take x`, `&hold x`
  - type forms: `&read[T, 'a]`, `&edit[T, 'a]`, `&take[T, 'a]`, `&hold[T, 'a]`
- Make this a surface-first plan only. Do not retarget `cabi`, `shackle`, or `arcana_winapi` in this plan. The deliverable is the language/runtime ownership model that later native work can target honestly.

## Public Surface Changes
- `read`, `edit`, `take`, and `hold` become the canonical general access modes.
  - `read`: shared observational access
  - `edit`: exclusive mutable access
  - `take`: consuming transfer
  - `hold`: owner-retained lifetime responsibility while direct-use exclusivity/freshness is suspended because outside interaction may occur
- Add capability-expression forms:
  - `&read x`
  - `&edit x`
  - `&take x`
  - `&hold x`
- Add capability-type forms:
  - `&read[T, 'a]`
  - `&edit[T, 'a]`
  - `&take[T, 'a]`
  - `&hold[T, 'a]`
- Remove the old canonical reference spellings from the language contract:
  - `&'a T`
  - `&'a mut T`
  - bare `&x`
  - bare `&mut x`
- Keep unary `*` as the uniform capability-use operator.
  - `*cap` projects through `&read` and `&edit`
  - `*cap` redeems `&take` as the one-shot consume step
  - `*cap` on `&hold` yields a temporary editable projection but does not end the hold
- Add statement-form `reclaim x`.
  - `reclaim` consumes an `&hold[...]` token and restores ordinary direct use of the original referent
  - `reclaim` is the only language-level hold-ending operation in this phase
- Rename owner exit `hold [...]` to `retain [...]`.
  - `retain [...]` remains the owner suspension/re-entry surface
  - `hold` is reserved for the new general access mode/capability family

## Key Semantic Changes
- Unify the ownership model so `&...` no longer defines a separate borrow universe.
  - `&read` and `&edit` are explicit reifications of the same access law already used by params and places
  - `&take` is a first-class deferred consume capability
  - `&hold` is a first-class retained-liveness capability
- Capability creation rules:
  - `&read x` creates a shared read capability
  - `&edit x` creates an exclusive mutable capability and suspends conflicting direct access
  - `&take x` reserves `x` immediately; direct use of `x` ends at token creation, and `*token` performs the later one-shot consume/redeem step
  - `&hold x` suspends direct use of `x` immediately; the original owner retains lifetime responsibility, and access is mediated through the hold token until explicit `reclaim`
- Capability use/copy rules:
  - `&read[...]` is duplicable/shared
  - `&edit[...]`, `&take[...]`, and `&hold[...]` are linear and non-duplicable
- Plain `hold` parameters are allowed.
  - `hold x: T` is an ordinary call-boundary access mode
  - it is ephemeral call hold only: caller direct use is suspended for the duration of the call
  - the callee cannot keep the hold past the call unless an explicit `&hold[...]` capability value is involved
- `&hold` lifecycle rules:
  - `*hold_cap` gives a temporary editable projection
  - deref does not end the hold
  - `reclaim hold_cap` ends the hold and restores ordinary direct use
  - live unreclaimed `&hold[...]` tokens are not allowed to silently escape scope end
  - unreclaimed hold tokens must be explicitly reclaimed or explicitly handled by cleanup/defer logic; otherwise they are a compile-time error when statically visible and a deterministic runtime error in remaining dynamic cases
- Cleanup and defer:
  - cleanup/defer may invoke `reclaim`
  - there is no implicit auto-reclaim fallback
- Owner model update:
  - existing owner suspension semantics remain, but use `retain [...]` instead of `hold [...]`
  - owner `retain [...]` is not the same surface as `&hold[...]`, though both express retained lifetime responsibility in different layers

## Implementation Changes
- Specs and frozen contract:
  - update `docs/arcana-v0.md`, access-modes, objects, and any related approved scope docs to make the new capability model authoritative
  - record this as an explicit pre-selfhost language-contract change rather than a backend-only cleanup
- Parser/HIR/frontend:
  - replace old borrow-expression parsing with `&read/&edit/&take/&hold`
  - replace old reference-type parsing with `&mode[T, 'a]`
  - remove `BorrowRead` / `BorrowMut` as distinct semantic primitives and re-express them in terms of the capability family
  - add `hold` to parameter/receiver access-mode validation
  - add `reclaim` statement parsing, lowering, typing, and diagnostics
  - rename owner exit `hold [...]` parsing/diagnostics to `retain [...]`
- Typechecking and ownership analysis:
  - rewrite place/borrow conflict checks around the four-mode capability lattice instead of special-casing legacy borrow syntax
  - preserve explicit place-based reasoning and lexical lifetime ties
  - enforce linearity for `&edit`, `&take`, and `&hold`
  - enforce immediate reservation for `&take`
  - enforce suspended direct use and explicit reclaim for `&hold`
- Runtime/IR:
  - replace the current ref-value behavior with capability values that distinguish `read/edit/take/hold`
  - make deref/redeem semantics explicit for each capability family
  - add runtime reclaim execution and deterministic unreclaimed-hold failure behavior
  - update owner runtime metadata and execution from `hold` to `retain`

## Test Plan
- Syntax/HIR tests:
  - parse and lower `&read/&edit/&take/&hold` expressions
  - parse and lower `&read[T, 'a]` / `&edit[T, 'a]` / `&take[T, 'a]` / `&hold[T, 'a]`
  - parse and lower `reclaim x`
  - parse and lower owner `retain [...]`
  - reject old `&'a T`, `&'a mut T`, bare `&x`, and bare `&mut x`
- Frontend/ownership tests:
  - `&read` duplicates successfully while `&edit`, `&take`, and `&hold` reject duplication
  - `&take x` immediately invalidates direct use of `x`
  - `*take_cap` consumes exactly once and invalidates the token afterward
  - `&hold x` immediately suspends direct use of `x`
  - `*hold_cap` gives temporary editable projection without ending hold
  - `reclaim hold_cap` restores ordinary direct use
  - unreclaimed hold tokens fail deterministically
  - plain `hold` params suspend caller direct use only for the call duration
  - owner `retain [...]` still supports suspension/re-entry behavior
- Runtime tests:
  - capability values execute with the same explicit semantics as the frontend model
  - `reclaim` works in normal control flow and inside `defer`/cleanup
  - unreclaimed hold tokens produce deterministic runtime errors in dynamic cases
- Regression tests:
  - existing `read/edit/take` call semantics remain intact
  - existing object/owner lifecycle still works after `hold [...]` -> `retain [...]`
  - existing struct/union/array/bitfield work remains intact under the new capability model

## Assumptions And Defaults
- This plan intentionally changes the pre-selfhost language contract and therefore requires corresponding scope/doc updates in the same patch series.
- `&read[...]`, `&edit[...]`, `&take[...]`, and `&hold[...]` are capability values, not C pointers and not numeric addresses.
- `read` remains duplicable; `edit`, `take`, and `hold` are linear.
- `&take` is included now because later CABI/native boundary work may need explicit delayed consume tokens, not just immediate `take`.
- `&hold` is general surface, not boundary-only, but its primary motivation is foreign/native lifetime mediation.
- `reclaim` is a statement-form ownership transition, not just cleanup timing; cleanup/defer may invoke it, but they do not define its meaning.
- The next plan after this one will retarget `cabi`/`shackle`/WinAPI to this capability model rather than expanding raw native transport against the old split borrow surface.
