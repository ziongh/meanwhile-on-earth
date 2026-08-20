# ADR 0004: Bounded, demand-driven materialization

- Status: accepted
- Date: 2026-08-19

## Decision

There is no global real-time simulation loop. Read requests carry finite patch, work, byte, and
instance budgets. Canonical cells/time segments are compiled offline; clients ask for state at an
instant and deterministically realize only the observation bubble.

Spatial aggregation, temporal resolution, semantic realization detail, and visual quality are
independent axes. Camera state affects only the last two.

## Consequences

Idle CPU is near zero and viewer count cannot change history. Compilation and checkpoint policy
become explicit product work. Requests may be downgraded or rejected rather than silently consuming
unbounded resources.
