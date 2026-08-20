# Unreal presentation client

This project is a thin, replaceable consumer of immutable semantic snapshots. It intentionally has
no direct Cesium, PCG, Mass, terrain, or lighting dependency yet. Those systems belong in
implementations of `IHistoryWorldPresentationAdapter`, not in canonical history or general gameplay
Blueprints.

The checked-in `.uproject` is the current UE 5.8 lane. Copy `engine-lock.example.json` to the
release pipeline's engine lock and replace all `REQUIRED_*` values with exact build/toolchain/plugin
checksums before cooking. Engine upgrades happen on a separate copy/branch and regenerate
presentation artifacts from the same semantic snapshot fixtures.

The subsystem already provides the two behaviors timeline scrubbing cannot add later as an
afterthought:

- each request pins a release digest;
- a newer request cancels and invalidates the older generation;
- byte, candidate-patch, deterministic-work, and suggested-instance budgets are sent to the service
  and checked again at the client boundary;
- contract version, digests, request identity, enum values, quantized integer ranges, provenance
  counts, and stable-time intervals are validated before a snapshot reaches presentation code;
- state remains quantized and provenance-labeled;
- rendering is delegated through a project-owned interface.

`UHistoryWorldSubsystem` is deliberately a focused-view request primitive, not the eventual globe
streamer: it owns one active request and timeline scrubbing cancels the previous generation. The
production streaming layer must sit above it (or replace its transport) with a priority queue keyed
by cell/layer/detail, bounded concurrency, request coalescing, and stable-time-segment cache
entries. Do not create one subsystem per visible cell.

The engine HTTP implementation buffers a completed body before this plugin can inspect it, and the
current decoder creates a UTF-16 string plus a JSON DOM on the game thread. The 2 MiB default and
server-side `max_output_bytes` limit make that prototype path finite, but they are not the final
streaming design. Before large regional payloads, move transport decoding and canonical validation
to a bounded worker task, publish compact native snapshot handles, and apply results incrementally
under the frame budget. Blueprint multicast delegates currently copy reflected snapshot arrays, so
reserve them for small prototype snapshots; high-volume presentation adapters should consume
immutable native handles.

The Unreal project cannot be compiled in this repository's current CI because the proprietary engine
toolchain is not installed there. The Rust/OpenAPI golden contract is the source for adapter
compatibility tests.
