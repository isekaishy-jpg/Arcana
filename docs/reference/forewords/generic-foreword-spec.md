# Generic Foreword Metadata Specification

Status: `reference-only`

## Purpose
Foreword metadata is a prefix metadata system for Arcana declarations. It carries structured metadata that can be validated and interpreted by a specific owner (compiler, tooling, runtime, or framework subsystem).

This document is the saved generic reference spec for future expansions beyond v1.
Current foreword contract lives in `docs/specs/forewords/forewords/v1-scope.md` and `docs/specs/forewords/forewords/deferred-roadmap.md`.

## Core idea
Forewords are attached before a declaration:

```text
#allow[dead_code]
#deprecated["Use new_fn"]
#only[os = "windows"]
```

Forewords are metadata attachments, not declaration body syntax.

## Canonical forms

```text
#name
#name[arg]
#name[arg1, arg2]
#name[key = value]
#name[arg1, arg2, key = value]
```

Forewords may stack on the same target.

## Conceptual model
Each foreword answers:
- what semantic category it belongs to
- who owns interpretation
- where it can attach
- when it takes effect
- why it exists
- how it is applied

## Categories
- Marker: `#test`, `#inline`
- Diagnostic policy: `#allow[...]`, `#deny[...]`
- Transform: `#derive[...]`
- Conditional inclusion: `#only[...]`
- Registration: framework-owned annotations
- Documentation/deprecation: `#deprecated[...]`
- Optimization hints: `#inline`, `#cold`

## Target model
Potential targets:
- file/module/import/use/reexport
- type and field
- function and parameter
- impl/trait members
- block/statement/expression

Each foreword definition declares valid targets.

## Ownership and phase model
Each foreword has a primary owner and phase:
- parse
- resolve
- analyze
- expand
- lower
- codegen
- runtime_init/runtime
- tooling

## Payload model
Payload schemas can be:
- none
- positional scalar
- positional list
- named fields
- mixed positional + named

Payload shape and value types must be validated.

## Definition model
A generic foreword definition conceptually includes:
- `name`
- `category`
- `owner`
- `targets`
- `phase`
- `intent`
- `payload_schema`
- `action`
- optional conflict/retention rules

## Validation rules
Diagnostics must cover:
- unknown foreword name
- invalid target
- malformed payload
- invalid payload type
- duplicate/forbidden combinations
- unavailable owner
- phase-incompatible usage

## Composition defaults
Forewords may stack. Default policy:
- order is not semantically significant unless explicitly defined by a foreword owner.

## Retention model
Retention classes:
- compile-time only
- tooling-visible only
- runtime-retained

Retention is per-foreword.

## Non-goals (generic)
This spec does not require:
- arbitrary AST rewriting
- mandatory runtime retention
- any specific backend/runtime architecture

## Open questions for future plans
- exact user-defined foreword syntax and visibility/import rules
- duplicate/conflict resolution defaults
- statement/expression target semantics
- introspection APIs and bytecode metadata carriage
