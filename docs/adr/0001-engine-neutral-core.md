# ADR 0001: Unreal is a replaceable presentation client

- Status: accepted
- Date: 2026-08-19

## Decision

Evidence, claims, deep-time coordinates, corpus releases, simulation rules, and semantic state use
engine-neutral schemas. Unreal consumes immutable semantic artifacts through HTTP/open formats and
maps them behind a project-owned adapter. There is no Rust-to-Unreal FFI in the initial
architecture.

`.uasset`, World Partition cells, Data Layers, PCG output, Mass entities, Nanite data, lighting
builds, cooked packages, and DDC are reproducible caches.

## Consequences

Unreal upgrades cannot corrupt the historical record, other clients remain possible, and
backend/engine crashes and release cadences are isolated. The cost is an explicit protocol plus
adapter and occasional duplicate DTO code. That cost is preferable to a long-lived C++ ABI and
engine-specific canonical data.
