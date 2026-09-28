# Architecture Audit Notes

This file records generated-source cache schema changes that must invalidate
previous compiler output.

## Cache schema

- `build-v95`: added typed arithmetic IR and native lowering for subtraction,
  multiplication, division, remainder, and unary negation; added string
  concatenation lowering and native collection `count`/`isEmpty` accessors;
  extended both dev interpreters and advanced the Dev IR and WebSocket protocol
  versions.
- `build-v94`: iterates Compose state collections through immutable snapshots.
- `build-v93`: binds Android runtime context before native plugin initialization.
- `build-v92`: includes development host File and Path helpers for hot reload.
