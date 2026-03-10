# Arcana Generic Host Platform Spec (Reference)

Status: `reference-only`

This is a broad saved direction note. Current host-platform contract lives in `docs/specs/selfhost-host/selfhost-host/v1-scope.md` and `docs/specs/selfhost-host/selfhost-host/deferred-roadmap.md`.

This document captures the broad host-platform direction for Arcana tooling grimoires.

## Purpose
- Enable Arcana-authored tools (lexer/parser/build tools) to interact with local host state deterministically.
- Keep host capability explicit and auditable.
- Preserve native-first behavior while retaining deterministic VM compatibility diagnostics.

## Capability Families
- Process context: args and environment reads.
- Filesystem and paths: bounded local file/directory operations.
- Text/bytes primitives: UTF-8 and byte-level helpers for parser/token tooling.

## Safety Model
- Host-root sandbox for filesystem mutation and reads.
- Host-root sandbox for process executable paths.
- Explicit capability gates for sensitive host actions (for example `--allow-process`).
- Deterministic, explicit diagnostics for unsupported/blocked operations.
- No ambient networking; host capabilities are explicit in CLI/runtime options.

## Non-Goals (Reference)
- Socket/network APIs.
- Full process orchestration surface (pipes, spawn handles, signals, PATH lookup).
- Full streaming I/O ecosystem (seek, buffering policies, async streams).

## Determinism Expectations
- Directory listings are lexicographically sorted.
- Error text is stable.
- Native execution is canonical for host APIs.
