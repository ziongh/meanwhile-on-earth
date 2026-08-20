# Performance and resource budgets

Performance is a correctness property: no request, zoom level, timeline speed, or contribution can
create unbounded CPU, memory, GPU, network, or storage work.

## Server

The current service is read-only, uses no GPU, has no background simulation loop, and idles on the
socket. Defaults are `min(logical CPUs, 4)` Tokio workers, `min(logical CPUs, 2)` compute/response
permits, a five-second compute timeout, 4,096 candidate patches, 100,000 work units, an 8 MiB
response bound, and 20,000 suggested instances. Environment controls are:

```text
HW_BIND
HW_WORKER_THREADS
HW_MAX_COMPUTE_TASKS
HW_SNAPSHOT_TIMEOUT_MS
HW_MAX_CANDIDATE_PATCHES
HW_MAX_RESPONSE_BYTES
HW_MAX_GENERATED_INSTANCES
HW_MAX_WORK_UNITS
RUST_LOG
```

Limits can be lowered operationally. Client values are clamped and cannot raise the server ceiling.
Candidate-only work is admitted before scanning; provenance work and output bytes are charged
incrementally before cloning. Materialization and JSON encoding run on a bounded blocking pool so
health and metrics remain responsive. A permit stays with each 64 KiB-chunked response body,
limiting slow-client response residency. Output also receives an exact encoded-size check. Requests
expose counters for patches, work units, errors, and bytes.

A production edge proxy must additionally enforce connection, header, idle-write, and total-request
timeouts. Those transport limits are deployment concerns and are not implied by the application
compute semaphore.

Production chunks should normally be 256 KiB–2 MiB compressed with an 8 MiB hard ceiling, tuned by
traces rather than folklore. Store compressed bytes once and serve them by digest/ETag; do not
recompress per viewer. Add bounded singleflight and negative caching before increasing concurrency.
All caches are byte-bounded, not entry-count-bounded.

At scale, the runtime path is object storage + CDN. Compute nodes materialize rare misses under
admission control and publish content-addressed immutable artifacts. The foundation's computed
snapshot route is deliberately `private, no-cache`: its URL does not yet include the complete
materialization profile, so only its ETag may be revalidated. PostgreSQL is the editorial source of
truth, not the frame-time serving database. Use a transactional outbox before adding Kafka; add
Redis only after a measured need.

## Simulation

- No global tick. Work is partitioned by layer, logical cell, and time segment.
- Precompute parent aggregates and sparse checkpoints; cap delta-chain length.
- Query only the cell ancestry and selected layers; production indexes add validity-segment lookup
  so history length does not imply scan length.
- Use fixed-point values and deterministic reductions. Parallel workers may process independent
  canonical partitions, but output merge order is fixed.
- Seed each subsystem/cell/epoch independently. Never share a global RNG.
- At 2,000× playback, sample visible states and meaningful transitions; never execute every skipped
  presentation frame.
- Camera and connected-viewer count cannot affect canonical computation.

## Unreal client

One resource governor coordinates otherwise independent streamers:

| Consumer             | Budget signals                                                          |
| -------------------- | ----------------------------------------------------------------------- |
| Cesium/3D Tiles      | screen-space error, cache bytes, simultaneous loads, culling            |
| World Partition/HLOD | focused local scene, loading radius, concurrent cells                   |
| PCG                  | generation radius, frame milliseconds, concurrent components, pooling   |
| Mass                 | actor/ISM/none representation LOD, simulation frequency, entity ceiling |
| Renderer             | frame time, VRAM, virtual texture/Nanite residency, quality profile     |
| Network              | in-flight regions, response bytes, cancellation generation              |
| Semantic cache       | resident bytes and pinned-release key                                   |

Stable/coarse procedural output is baked offline. Runtime PCG is near-camera, hierarchical, pooled,
cancellable, and usually emits ISM/HISM/Nanite-friendly batches rather than thousands of Actors. GPU
PCG is cosmetic and must retain a CPU/baked fallback; GPU floating-point differences cannot alter
historical state.

Mass entities exist only in the observation bubble. Representation transitions from full Actor to
low-detail Actor to instances to none, with lower simulation frequency at distance. Synthetic
pedestrians are not serialized or replicated as durable people.

Every camera/time request carries a generation ID. HTTP, tile loads, procedural generation, and
agent realization check it before committing work. Timeline scrubbing cancels stale generations
immediately.

## Storage and perpetuity

- Raw/source assets are archival, checksummed, replicated, and retained in open or source-quality
  formats.
- Canonical features are stored once; cell coverage is an index.
- State uses temporal segments/deltas plus checkpoints, never a full planet copy per year.
- Cooked Unreal packages, DDC, HLODs, light builds, generated GLB/3D Tiles, and caches carry full
  input/build/profile keys and are safe to delete.
- Dependency-aware garbage collection retains everything reachable from a published release and
  observes licensing/retention policy.

## Gates

Each representative scene and service workload gets a checked-in budget envelope for p50/p95/p99
latency, requests/CPU-second, peak/request RSS, response bytes, cache hit rate, frame time,
game/render/RHI thread time, VRAM, network, and storage amplification. Regressions require an
explicit reviewed budget change.

`cargo bench -p historyworld-sim` is the first microbenchmark. Required future soak tests include
pole/antimeridian travel, repeated origin shifts, rapid timeline scrubbing, region load/unload leak
detection, cold/warm cache behavior, and deterministic replay across worker counts.
