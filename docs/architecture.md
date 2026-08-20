# Architecture

## Goal

Support a perpetual, collaboratively improved historical world without coupling its evidence or
semantic state to one database, game engine version, renderer, or server topology.

The system is a pipeline, not one monolithic simulation process.

## Four planes

### 1. Knowledge and authoring

PostgreSQL/PostGIS stores lightweight source metadata, immutable source/claim revisions, temporal
and spatial uncertainty, review/moderation events, and release manifests. Original documents, scans,
rasters, models, and large probability datasets live in content-addressed object storage.

This plane favors auditability over runtime speed. Hot predicates are projected from generic claims
into versioned typed read models. Runtime clients never scan arbitrary contribution JSON.

### 2. Compilation and simulation

A corpus release freezes the admitted knowledge graph at a moderation sequence. An interpretation
revision selects hypotheses and reconstruction policies without rewriting that corpus. A pinned
compiler turns both into sparse, layer-specific, space-time chunks and checkpoints.

Canonical work uses fixed-point/integer quantities, stable sorting, explicit reduction order, and
independently derived random streams. GPUs may accelerate disposable rendering or approximate
offline experiments; GPU results never silently become authoritative semantic history.

### 3. Runtime delivery

The normal read path is immutable object delivery:

1. resolve a mutable world head once;
2. pin its release digest;
3. fetch small manifests;
4. stream only requested layers/cells/time segments;
5. reuse them until a real validity boundary is crossed;
6. cancel work made stale by camera or timeline movement.

Popular artifacts are prebuilt and CDN-cached. Rare materializations execute under explicit patch,
work, byte, memory, and wall-time limits. Precompressed objects are served as-is; the server does
not repeatedly spend CPU compressing the same historical state.

### 4. Presentation

Unreal receives semantic constraints and renderer-neutral assets, then chooses a capability profile
for the current platform. It owns camera, interaction, audio, animation, atmosphere, local
collision/navigation proxies, and ephemeral visual realization. It does not own evidence,
world-valid time, simulation truth, or the only copy of source assets.

The initial C++ plugin depends only on HTTP/JSON and a project-owned presentation interface. Cesium,
World Partition, PCG, Mass, terrain, Nanite, Lumen, and future features stay behind adapters.

## Space

Canonical geometry retains its CRS/reference frame, ellipsoid, vertical datum, coordinate epoch,
accuracy, and—where deep time requires it—plate and reconstruction-model revision. Rendering derives
ECEF and a local tangent frame near the active camera. Unreal centimeters, `FVector`, rebased
origins, and Cesium tile IDs are never persisted as truth.

The first logical index is `hw-cube-sphere-v1`: six roots, a quadtree path, and 28 levels packed
into 64 bits. It handles poles and the antimeridian without Web Mercator's singularity. The scheme
is tagged and replaceable; original geometry is retained so coverage can be re-indexed.

Global terrain/imagery and local authored scenes have separate ownership:

- Cesium/3D Tiles streams the global visual surface.
- One focused local tangent scene uses World Partition/HLOD for authored content.
- A local collision/nav proxy supports gameplay; streamed global terrain is not assumed to remain
  resident or be a durable physics surface.

## Time

`WorldInstant` is signed 64-bit seconds plus nanoseconds. This spans roughly ±292 billion years and
safely covers Earth history. Calendar strings such as “circa 10,000 BP” remain source metadata with
calendar/conversion-model version; they are not discarded after conversion. Intervals are
start-inclusive and end-exclusive.

Spatial cells and time do not form a giant dense cube. Each layer chooses its own resolution and
stores sparse deltas plus capped chains and periodic checkpoints. Adjacent identical segments are
coalesced. A geology cell may be stable for millennia while a watched battle has subsecond
presentation updates.

## Multiscale semantics

Four axes are negotiated independently:

| Axis                 | Examples                           | Canonical?                       |
| -------------------- | ---------------------------------- | -------------------------------- |
| Spatial aggregation  | planet, basin, region, block       | Yes; fixed simulation partitions |
| Temporal resolution  | epoch, year, day, second           | Yes; layer/ruleset policy        |
| Semantic realization | aggregate, feature, cohort, person | Derived deterministically        |
| Visual quality       | mobile, desktop, cinematic         | Disposable presentation          |

The camera controls realization and visual quality, never canonical partitioning. Cross-cell
features have one owner and read-only halo data. Parent aggregates are precomputed and tested
against children; interactive requests do not merge thousands of fine cells.

Synthetic people are short-lived samples of conserved population cohorts. Only historically attested
identities or noteworthy semantic events need persistent records. Reabsorption removes render
entities, not people from the aggregate.

## Unreal today and tomorrow

Current official guidance supports this boundary:

- [World Partition](https://dev.epicgames.com/documentation/en-us/unreal-engine/world-partition-in-unreal-engine)
  and
  [HLOD](https://dev.epicgames.com/documentation/en-us/unreal-engine/world-partition---hierarchical-level-of-detail-in-unreal-engine)
  are local authored-content streamers, not a historical database.
- [Large World Coordinates](https://dev.epicgames.com/documentation/en-us/unreal-engine/large-world-coordinates-in-unreal-engine-5)
  and
  [georeferencing](https://dev.epicgames.com/documentation/en-us/unreal-engine/georeferencing-a-level-in-unreal-engine)
  help precision, but local frames/origin-shift tests remain mandatory.
- [Data Layers](https://dev.epicgames.com/documentation/en-us/unreal-engine/world-partition---data-layers-in-unreal-engine)
  represent a small number of materially distinct authored phases—not one layer per year.
- [PCG generation modes](https://dev.epicgames.com/documentation/en-us/unreal-engine/using-pcg-generation-modes-in-unreal-engine)
  consume semantic constraints and explicit seeds. Stable/coarse work is baked; near-camera cosmetic
  detail is bounded and cancellable.
- [Mass Entity](https://dev.epicgames.com/documentation/en-us/unreal-engine/overview-of-mass-entity-in-unreal-engine)
  is only the active observation bubble's representation of aggregates.
- [Cesium georeferenced sublevels](https://cesium.com/learn/unreal/unreal-sublevels/) support one
  focused local frame, while 3D Tiles handle global streaming.

An engine lock pins the UE build/commit, compiler/SDK, every plugin digest, platform/RHI,
schema/generator versions, and DDC namespace. `engine-next` upgrades convert a copy because Unreal
conversions are one-way. Gates include semantic golden hashes, pole/antimeridian/origin tests, asset
load/cook/package, screenshot tolerances, fly/scrub soak tests, unload leak checks, and
CPU/GPU/RAM/VRAM/network budgets. Epic's
[upgrade guidance](https://dev.epicgames.com/documentation/en-us/unreal-engine/updating-projects-to-newer-versions-of-unreal-engine)
is the baseline process.

When a better lighting or terrain system arrives, a capability profile consumes the same physical
material, atmosphere, solar, climate, terrain, and semantic inputs. Only the render key and
regenerated artifact change.

## Dependency direction

```text
historyworld-domain
       ▲
historyworld-content
       ▲
historyworld-sim
       ▲
 CLI / HTTP adapters ───── open wire formats ───── Unreal adapter
```

The domain knows nothing about SQL, async runtimes, HTTP, Protobuf, Unreal, or Cesium.
Materialization accepts compiled values rather than issuing database queries. This keeps tests
deterministic and makes future storage/service splits an operational choice instead of a rewrite.
