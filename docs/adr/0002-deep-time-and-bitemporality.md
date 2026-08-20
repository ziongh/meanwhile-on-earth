# ADR 0002: Signed-second deep time and separate repository time

- Status: accepted
- Date: 2026-08-19

## Decision

Canonical world instants are signed 64-bit seconds from the versioned Unix epoch plus unsigned
nanoseconds. Intervals are start-inclusive/end-exclusive. Calendar conversion and original source
expressions are versioned metadata.

Repository history uses a separate monotonic system sequence plus a modern audit timestamp.
PostgreSQL timestamp, JavaScript number, Unreal datetime, and signed nanoseconds are not canonical
world-time encodings.

## Consequences

The representation covers Earth history with subsecond precision and serializes seconds as decimal
strings in JSON. Consumers must use explicit calendar helpers. Uncertain extents need multiple
bounds instead of a convenient single date.
