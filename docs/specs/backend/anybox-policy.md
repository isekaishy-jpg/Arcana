# AnyBox Policy

Status: `approved-pre-selfhost`

`AnyBox`-style erased Arcana values are banned from the rewrite contract.

## Policy

- No public source-language `Any` or `AnyBox` type.
- No typed HIR node, package interface, or IR value class based on an erased Arcana value carrier.
- No backend ABI handle kind whose meaning is "arbitrary Arcana value".
- No lowering fallback that boxes unsupported values into an erased handle just to keep compilation moving.

## Rationale

- Previous implementation work hit backend and lowering friction around erased value carriers.
- Erased fallback values hide type mistakes instead of forcing explicit contracts.
- Selfhost and AOT work are easier to reason about when all Arcana values stay typed through the pipeline.

## Required Rule For Rewrite Work

- If a feature cannot be lowered without an `AnyBox`-style escape hatch, that feature stays unsupported until typed lowering exists.
- Backend convenience is not a sufficient reason to reintroduce erased Arcana value carriers.

## Scope Boundary

- Ordinary implementation-language internals in Rust may still use local dynamic utilities where needed.
- What is banned is an Arcana semantic/runtime value class that crosses typed HIR, IR, runtime ABI, host ABI, or first-party library boundaries as an erased value carrier.

## Review Checklist

- No `AnyBox`, `HandleKind::Any`, or equivalent typed-erasure concept in new runtime or backend code.
- No package/build/cache format depending on erased Arcana values.
- No host/tooling API requiring value erasure instead of typed rows or named records.
