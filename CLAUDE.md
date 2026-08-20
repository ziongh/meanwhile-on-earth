# HistoryWorld agent guide

This is the operating manual for AI agents and contributors working in this repository. Read it
before editing. It explains what exists, where a change belongs, which invariants cannot be
weakened, and how to take one domain from evidence to a bounded Unreal realization.

Keep this file current whenever repository structure, canonical contracts, commands, or contribution
lanes change. Do not turn it into a duplicate of every design document: link to the durable detail
and record here the decisions an agent needs in order to act safely.

## Mission

HistoryWorld is an evidence-aware, continuously improvable historical globe. A viewer should be able
to move across the planet and through time, from geological eras to regions, settlements, streets,
and eventually individual lives, while always being able to distinguish:

- what is directly observed or documented;
- what is reported by a primary source;
- what scholars infer;
- what a versioned model derives;
- what is a plausible reconstruction; and
- what is artistic ambient detail.

The project is meant to improve perpetually, more like an encyclopedia than a game with a final
content release. Historical corrections append new evidence, claims, interpretations, and releases;
they do not erase the views that earlier releases represented. Renderer and Unreal upgrades should
make the same semantic world look better without rewriting historical truth.

The first product proof remains deliberately narrow: Vesuvius and Pompeii around 60, 78, 79, and 100
CE, from an aerial view to a local/street view, including the eruption boundary and visible
provenance. The checked-in Vesuvius data is only a synthetic `artistic_ambient` architecture
fixture, not historical research.

## Current maturity: foundation, not finished world

Implemented now:

- deep-time, uncertainty, stable-ID, content-hash, provenance, resource-budget, and globe-cell
  types;
- a deterministic compiler for immutable semantic release documents;
- a bounded, on-demand materializer for sparse surface state;
- a verified local filesystem content-addressed store;
- a local CLI and a bounded read-only HTTP service over the bundled fixture;
- OpenAPI and Protobuf contracts;
- an initial append-only PostgreSQL 15+/PostGIS authoring schema; and
- a UE 5.8 project with a project-local, code-only runtime plugin and a bounded decoder for its
  currently reflected snapshot subset.

The layer contract declares `geology`, `hydrology`, `climate`, `biome`, `population`, `settlement`,
`political`, and `event`. Only the first four currently have materialized fields/patch operations:

- geology: mean elevation, relief, and land fraction;
- hydrology: water fraction;
- climate: mean temperature and annual precipitation; and
- biome: vegetation fraction.

Population, settlements/cities, politics, wars, people, disasters, and other events are
architectural lanes, not implemented simulations. There is also no authoring application, production
release builder, database-to-compiler ingestion path, S3/CDN adapter, Cesium/PCG/Mass integration,
globe streamer, 3D asset pipeline, authentication, or production deployment yet. Never describe
planned systems as present.

## Read before changing code

Use the following sources together:

1. [README.md](README.md) gives the runnable foundation and short orientation.
2. [Architecture](docs/architecture.md) defines the four planes, space/time model, engine boundary,
   and multiscale semantics.
3. [Data model](docs/data-model.md) defines evidence, claims, bitemporality, releases, and storage
   roles.
4. [Performance budgets](docs/performance-budgets.md) defines the CPU, memory, GPU, network, and
   storage policy.
5. [Roadmap](docs/roadmap.md) gives the intended dependency order.
6. [Accepted ADRs](docs/adr/) are architectural decisions. Add an ADR rather than silently reversing
   one.
7. OpenAPI, Protobuf, SQL migrations, Rust tests, and Unreal DTO validation are executable contracts
   for their respective boundaries.

If these disagree, do not pick the most convenient version. Identify whether the code, wire
contract, or design is stale, then reconcile the affected documents, schemas, implementation, and
tests in one intentional change.

Preserve the user's worktree. Inspect `git status` before editing, do not remove or overwrite
unrelated changes, do not commit or push unless asked, and keep changes scoped to the requested
outcome.

## System model

HistoryWorld is a pipeline, not a monolithic game process:

```text
sources / scans / measurements / scholarship
                       |
                       v
immutable source revisions -> citations -> claim revisions -> review/moderation
                       |
                       v
              immutable corpus release
                       |
             interpretation revision
                       |
                       v
        pinned deterministic compiler/materializer
                       |
                       v
sparse semantic chunks + checkpoints + exact dependency manifests
                       |
             HTTP / Proto / glTF / 3D Tiles
                       |
                       v
       replaceable Unreal/presentation adapters
                       |
                       v
 meshes / PCG / Mass / audio / lighting / animation / UI
```

The four operational planes are:

1. **Knowledge and authoring.** PostgreSQL/PostGIS stores lightweight metadata, geometry, immutable
   revisions, uncertainty, workflow events, and release manifests. Large originals belong in
   content-addressed object storage.
2. **Compilation and simulation.** A pinned corpus plus interpretation compiles offline into sparse,
   typed, layer/cell/time artifacts. Canonical math is deterministic, fixed-point/integer, ordered,
   and resource-bounded.
3. **Runtime delivery.** Resolve a mutable head once, pin the release digest, and fetch only the
   required immutable manifests/chunks. Popular artifacts come from object storage/CDN; rare compute
   is bounded and rejectable.
4. **Presentation.** Unreal owns camera, interaction, animation, sound, lighting, atmosphere,
   collision/navigation proxies, and disposable visual realization. It does not own evidence or the
   only copy of semantic state/source assets.

Keep these persistence classes distinct:

| Class                       | Examples                                                                 | Policy                                 |
| --------------------------- | ------------------------------------------------------------------------ | -------------------------------------- |
| Irreplaceable knowledge     | sources, rights, citations, claim/review history, releases               | checksummed, audited, replicated       |
| Rebuildable semantic output | typed projections, chunks, checkpoints, dependency manifests             | pinned and reproducible for a release  |
| Disposable presentation     | `.uasset`, HLOD, PCG/Mass output, meshes, collision, lighting, cook, DDC | regenerate from semantic/source inputs |

Dependency direction is intentional:

```text
historyworld-domain -> historyworld-content -> historyworld-sim -> CLI / HTTP
          |
          +-------> historyworld-storage --------------------------> CLI

CLI / HTTP -------------------- open wire formats -----------------> Unreal adapter
```

The domain crate knows nothing about SQL, async runtimes, HTTP, Protobuf, Unreal, or Cesium. The
materializer accepts a `CompiledRelease`; it does not query authoring storage. There is no
Rust/Unreal FFI. Preserve this direction unless a new ADR justifies a replacement architecture.

## Repository map

| Path                            | Responsibility                                                                                   |
| ------------------------------- | ------------------------------------------------------------------------------------------------ |
| `apps/historyworld-cli`         | Validate/inspect releases, materialize snapshots, demo-publish local CAS objects, verify objects |
| `apps/historyworld-server`      | Bounded GET-only HTTP adapter over the currently compiled-in fixture                             |
| `crates/historyworld-domain`    | Engine-neutral time, space, IDs, layers, provenance, and budgets                                 |
| `crates/historyworld-content`   | Fail-closed release parsing, canonical hashing, validation, and indexes                          |
| `crates/historyworld-sim`       | Pure demand materialization, local hashes/seeds, golden test, benchmark                          |
| `crates/historyworld-storage`   | Synchronous object-store port and verified filesystem CAS                                        |
| `data/fixtures`                 | Explicit demo/compatibility inputs, never unlabeled research truth                               |
| `infra/postgres/migrations`     | Append-only PostgreSQL 15+/PostGIS authoring schema                                              |
| `schemas/openapi`               | JSON/HTTP control-plane contract                                                                 |
| `schemas/proto/historyworld/v1` | Versioned compact-binary evolution contract; bindings are not generated yet                      |
| `unreal/HistoryWorldClient`     | UE 5.8 presentation project and project-local runtime plugin                                     |
| `docs/adr`                      | Accepted architecture decisions                                                                  |
| `docs`                          | Architecture, data/provenance model, budgets, and roadmap                                        |
| `.github/workflows`             | Lightweight Rust/schema checks; no Unreal or live PostGIS lane yet                               |
| `.vscode`, `.zed`               | Project editor recommendations/settings/tasks that delegate to the Makefile                      |

Fast routing inside those packages:

| Change                                                                   | Start here                                                                            |
| ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------- |
| Deep time or uncertainty                                                 | `crates/historyworld-domain/src/time.rs`                                              |
| Geodesy, reference frames, or cells                                      | `crates/historyworld-domain/src/space.rs`                                             |
| Layer masks or current surface fields                                    | `crates/historyworld-domain/src/layer.rs`                                             |
| Evidence/confidence/epistemic types                                      | `crates/historyworld-domain/src/provenance.rs`                                        |
| IDs and content hashes                                                   | `crates/historyworld-domain/src/id.rs`                                                |
| Detail/resource budgets                                                  | `crates/historyworld-domain/src/budget.rs`                                            |
| Release documents, patch operations, canonical hash grammar              | `crates/historyworld-content/src/lib.rs`                                              |
| Snapshot materialization, local state hashes, seeds, work/byte admission | `crates/historyworld-sim/src/lib.rs`                                                  |
| Filesystem CAS and object-store contract                                 | `crates/historyworld-storage/src/lib.rs`                                              |
| CLI behavior                                                             | `apps/historyworld-cli/src/main.rs`                                                   |
| HTTP behavior                                                            | `apps/historyworld-server/src/main.rs`                                                |
| JSON wire contract                                                       | `schemas/openapi/historyworld-v1.yaml`                                                |
| Binary wire contract                                                     | `schemas/proto/historyworld/v1/world.proto`                                           |
| Authoring persistence                                                    | `infra/postgres/migrations/` and `docs/data-model.md`                                 |
| Unreal transport/decoder/settings                                        | `unreal/HistoryWorldClient/Plugins/HistoryWorldRuntime/Source/HistoryWorldRuntime/`   |
| Unreal presentation integration                                          | implement `IHistoryWorldPresentationAdapter` in a project-owned adapter module/plugin |

Put unit tests beside Rust implementation in `#[cfg(test)]` modules. Put cross-module compatibility
tests under a crate's `tests/`, materializer benchmarks under `crates/historyworld-sim/benches/`,
and versioned demo/golden inputs under `data/fixtures/`. Add new ADRs as the next numbered
`docs/adr/NNNN-short-name.md`.

## Toolchain and commands

Run commands from the repository root. Rust 1.91.1 is pinned by `rust-toolchain.toml`; Node 22 by
`.nvmrc`; npm dependencies by `package-lock.json`; Buf and Actionlint versions by `Makefile`. Go is
used to run the pinned Buf/Actionlint tools. `clang-format` and Unreal are separate prerequisites
and are not installed by repository setup.

Start a new checkout with:

```bash
make setup
make ci
```

`make setup` runs `npm ci --ignore-scripts`; it does not install Rust, Node, Go, Unreal, or
ClangFormat. The first Go-backed check may download its pinned tool.

Use the Makefile as the command API:

| Command                    | Purpose                                                                                  |
| -------------------------- | ---------------------------------------------------------------------------------------- |
| `make ci`                  | Format checks, Rust check/Clippy/tests, Markdown/Actions/schema lint, fixture validation |
| `make check`               | `cargo check --workspace --all-targets --locked`                                         |
| `make test`                | All workspace tests with the lockfile                                                    |
| `make lint`                | Rust, Markdown, workflow, OpenAPI, and Protobuf lint/build                               |
| `make schema-check`        | OpenAPI lint plus Protobuf lint/descriptor build                                         |
| `make validate`            | Compile and validate the bundled release                                                 |
| `make format`              | Apply Rustfmt, Prettier, and Buf formatting                                              |
| `make format-check`        | Check Rust, text, and Protobuf formatting                                                |
| `make format-unreal`       | Apply root ClangFormat policy to Unreal C/C++ source                                     |
| `make format-check-unreal` | Check Unreal C/C++ formatting; requires `clang-format`                                   |
| `make run-server`          | Run the read service on the configured bind address                                      |
| `make demo`                | Materialize the default year-80 fixture snapshot                                         |
| `make bench`               | Run the deterministic materializer microbenchmark                                        |

`make ci` is serial by design. It does not format/build Unreal, start PostGIS, execute migrations,
or run a production asset pipeline. `make format` does not format SQL migrations or Unreal C++.
Published migrations are historical records and must not be bulk-formatted. `CLAUDE.md` is excluded
from the normal Prettier/Markdownlint globs, so review its Markdown manually.

Useful focused commands are:

```bash
cargo test --locked -p historyworld-domain
cargo test --locked -p historyworld-content
cargo test --locked -p historyworld-sim --test vesuvius_golden
cargo test --locked -p historyworld-server

cargo run --locked -p historyworld-cli -- --help
cargo run --locked -p historyworld-cli -- validate path/to/release.json
cargo run --locked -p historyworld-cli -- manifest path/to/release.json
cargo run --locked -p historyworld-cli -- snapshot \
  --at-year 79 --month 8 --day 23 --detail local
```

The CLI snapshot defaults to cell `0:14:ee085cd`, local detail, all layers, and the interactive
budget. Local publishing is demonstrative only:

```bash
cargo run --locked -p historyworld-cli -- publish \
  --data-dir ./artifacts --input path/to/release.json
cargo run --locked -p historyworld-cli -- verify-object \
  --data-dir ./artifacts --hash BLAKE3_DIGEST
```

That publisher does not prove evidence/dependency closure and must not be presented as production
publication.

For the server:

```bash
make run-server
curl -s http://127.0.0.1:8080/healthz
curl -s http://127.0.0.1:8080/v1/releases/current
```

The process does not load `.env` automatically. Its environment variables are documented in
`.env.example` and [performance budgets](docs/performance-budgets.md). It currently serves only the
checked-in fixture; it does not load PostgreSQL or `./artifacts`.

For materializer changes, run `make bench` and report the environment as well as before/after time,
allocations/RSS where available, and input shape. A local microbenchmark number is not a universal
regression threshold.

## Non-negotiable invariants

1. **Unknown is not zero.** Unknown, absent, contradicted, inferred, reconstructed, and artistic are
   distinct. Optional semantic values use presence/null explicitly.
2. **Provenance survives compilation.** Every visible value leads to exact claim/evidence revisions,
   a pinned model derivation, a plausible reconstruction, or an artistic label. Only
   `artistic_ambient` patches may omit claims/evidence.
3. **World time and repository time are separate.** Historical validity never uses repository audit
   timestamps, and a new contribution never rewrites an old release.
4. **No artifact internally means `latest`.** Pin release, corpus, interpretation, dependency
   hashes, clock/cell/compiler/materializer/numeric/PRNG versions, seeds, and effective profiles.
5. **Canonical state is engine-neutral.** Unreal/Cesium coordinates, actors, tiles, Data Layers,
   Blueprint values, PCG graphs, Mass entities, lighting, and cooked assets are never truth.
6. **Determinism is explicit.** Use fixed-point/integer quantities, checked arithmetic, total
   ordering, fixed merge/reduction order, and independent versioned random streams.
7. **Camera and hardware cannot change history.** GPU, thread scheduling, frame rate, viewer count,
   camera path, renderer quality, and requested visual detail cannot alter semantic state hashes.
8. **Zoom conserves quantities.** Population, mass, land use, inventories, and flows obey declared
   parent/child aggregation rules. Synthetic people are samples of cohorts, not additional people.
9. **Work is finite before it starts.** Every request, compile, import, queue, cache, and
   realization has count/work/byte/time limits, admission before allocation,
   cancellation/backpressure, and metrics appropriate to its risk.
10. **There is no global tick or dense space-time cube.** Work is sparse by layer, canonical cell,
    and stable time segment. Offline checkpoints/aggregates make reads proportional to local
    dependencies, not total history.
11. **Every presentation cache is disposable.** Deleting DDC, HLODs, generated meshes, PCG/Mass
    output, or cooked packages must not delete knowledge.
12. **Compatibility changes are versioned.** Never reuse/reinterpret a field, enum ordinal, mask
    bit, canonical tag, Protobuf field number, time/cell scheme, seed namespace, or digest.

## Standard contribution workflow

First classify the change:

- **Evidence/content:** new source, citation, claim revision, uncertainty, hypothesis, or moderation
  event.
- **Semantic model:** a new typed field, feature, operation, aggregation rule, transition, or
  derived event consequence.
- **Core/contract:** time, cells, IDs, hashing, ordering, releases, PRNG, budgets, storage, SQL, or
  wire semantics.
- **Presentation:** Unreal adapter, renderer profile, mesh/material/animation/audio,
  PCG/Mass/Cesium, UI, or other disposable realization.

Then build the smallest complete vertical slice:

1. State the historical/scientific question and what remains unknown.
2. Identify exact source revisions, rights, citations, spatial role/reference frame, temporal
   uncertainty, epistemic class, and interpretation policy.
3. Define engine-neutral units, validation, stable identity, aggregation/conservation, layer,
   spatial resolution, temporal cadence, and ownership of cross-cell features.
4. Define all causal inputs and outputs. Cross-layer feedback must be a pinned model/run, never
   hidden mutation caused by query or rendering order.
5. Budget compiler work, request CPU, allocations/RSS, encoded/network bytes, storage growth, and,
   for presentation, game/render thread time and VRAM/GPU work.
6. Implement from the lowest dependency layer outward: domain -> content -> sim ->
   adapters/contracts -> Unreal presentation.
7. Add validation at trust boundaries, focused unit tests, compatibility fixtures/goldens, malformed
   input tests, and a benchmark/budget test proportional to risk.
8. Add an ADR, version/schema decision, append-only migration, and regeneration story when canonical
   compatibility changes.
9. Run targeted tests, `make format`, `make ci`, and the separate Unreal/PostGIS/asset lane when
   touched. Report unavailable toolchains rather than implying they ran.

A new noun does not automatically deserve a new layer. Prefer a predicate/entity or typed projection
inside an existing layer. Add a layer only when it has independently useful cadence, resolution,
streaming, querying, budgeting, and invalidation semantics, and coordinate the enum/mask across
Rust, OpenAPI, Proto, SQL projections, and Unreal.

## Domain dependency and scheduling

A typical dependency flow is:

```text
reference frames / plate reconstruction / geology
                         |
                         v
              terrain + coast + hydrology
                         |
                         v
                 climate + soils
                         |
                         v
              biome + vegetation state
                         |
                         v
      land use + settlement + infrastructure
                         |
                         v
        population + production + trade/flows
                         |
                         v
              political state + events
```

Real systems have feedback loops—people clear vegetation, wars damage cities, eruptions move people,
and climate affects food. Resolve those through an explicit, versioned offline ruleset with declared
time steps, input checkpoint, update order, and output manifest. Do not let a renderer callback or
unordered runtime loop create causal semantics.

An event records that an episode occurred. Its durable effects are typed transitions in the affected
layers. An eruption animation is not the eruption; a destroyed mesh is not canonical devastation;
and a moving border material is not political control.

## Contributing geometry and geodesy

Geometry is a cross-cutting foundation, not a renderer layer.

- Preserve raw source geometry, CRS/reference-frame authority and revision, ellipsoid, vertical
  datum, coordinate epoch, accuracy/envelope, and reconstruction-model hash.
- Use fixed-point semantic units. `GeoPointE7` stores latitude/longitude in `1e-7` degrees and
  height in millimetres; a `SpatialReference` supplies its meaning.
- `WorldCell` is an index, not geometry. `hw-cube-sphere-v1` has six faces and 28 quadtree levels in
  a `u64`; text is `face:level:lowercase-hex-path`.
- The v1 point projection uses floating trigonometry and pinned boundary rules, so perform it in a
  pinned offline compiler. Retain source geometry to rebuild a future scheme.
- Cross-cell features have one canonical owner and read-only halo/coverage indexes. Do not duplicate
  a river, route, wall, or structure as multiple canonical features simply because it crosses tiles.
- Unreal centimetres, `FVector`, rebased origins, Cesium tile IDs, ECEF/local tangent coordinates,
  and World Partition cells are adapter outputs only.

Changes need pole-longitude normalization, antimeridian, all face/corner ties, level-28, raw/text
round-trip, parent/child, cross-target projection goldens, seams, local-frame conversion, and
repeated origin-shift tests. Add a new tagged cell scheme rather than modifying v1 in place.

## Contributing geology and terrain

Canonical geology includes evidence/model-backed elevation, relief, land fraction, morphology,
deposits, faults/volcanism, plate/reconstruction identity, and uncertainty at an appropriate spatial
and temporal resolution. Terrain meshes are not canonical geology.

The current vertical slice supports `SetMeanElevationMm`, checked `OffsetMeanElevationMm`,
`SetReliefMm`, and `SetLandFractionPpm`. Extend it only with explicit units, range/cross-field
validation, canonical operation tags, totality proofs, layer hashing, and source/model provenance.

For paleogeography or terrain:

- retain licensed raw elevation/bathymetry/geology inputs in CAS rather than PostgreSQL or Git;
- pin plate, sea-level, interpolation, resampling, and reconstruction models;
- compile sparse validity segments/checkpoints at geological cadence instead of annual copies;
- precompute parent aggregates and test child conservation and cell seams; and
- derive heightmaps, triangulation, normals, collision, Nanite, Cesium/3D Tiles, and World Partition
  artifacts under a renderer profile.

## Contributing hydrology, sea, and climate

Hydrology and climate remain separate because their cadence, data, dependencies, and rendering
differ. Water fraction, mean temperature, and annual precipitation are only the foundation
fields—not a full ocean, drainage, or weather model.

- Hydrology may add sea level, coastline/wetness, watersheds, rivers, lakes, aquifers, flow, and
  flood envelopes as typed values/features with topology, units, uncertainty, and model version.
- Climate may add temperature/precipitation envelopes, seasonality, wind, aridity, or extreme-event
  statistics with an explicit temporal aggregation window and scenario/model inputs.
- Soil may be a typed environmental/biome projection or a new layer only if independent streaming
  and cadence justify it.
- Boundary conditions and outputs must be deterministic and partitionable. A GPU weather/ocean
  simulation is visual or experimental unless its output is imported through a pinned reproducible
  derivation.
- Waves, shaders, clouds, rain, fog, and momentary local weather are presentation unless a semantic
  event/state explicitly requires them.

Test coastline/river continuity, basin conservation, parent aggregation, stable intervals, missing
data, model/scenario switches, and local rendering seams.

## Contributing vegetation and ecology

`Biome` owns semantic cover, composition, succession, ecological constraints, and disturbances.
`vegetation_fraction_ppm` is a coarse cover constraint; it does not mean that individual trees have
been observed.

- Preserve taxonomy/biome definitions, date and spatial uncertainty, sampling bias, climate/soil
  model dependencies, and human land-use effects.
- Represent clearing, succession, fire, disease, desertification, and regrowth as versioned changes
  with explicit causes and validity; do not bake them into a static forest mesh.
- Human land use belongs in settlement/land-use semantics even when it changes biome state.
- Generate plant instances from the local dependency hash, cell, stable epoch, species/biome rule,
  and ordinal. Independent namespaces prevent one subsystem's random draws from reshuffling another.
- Individual vegetation is disposable PCG/instance output unless a specific tree is historically
  attested as a stable feature.

Use instancing, hierarchical LOD, bounded density, cancellation, and platform-specific GPU/VRAM
budgets. Test aggregate cover/composition conservation across LOD and deterministic regeneration.

## Contributing settlements, cities, and urbanization

The `Settlement` layer is declared but not implemented. Its future typed projections may include
settlement extents/status, archaeological features, structures, districts, roads, ports, utilities,
land use, production, and trade infrastructure.

- Give canonical features stable identity and store their source geometry once. Names, function,
  ownership, condition, dimensions, and dates are claims/revisions, not mutable identity fields.
- Model a city through meaningful phases and sparse events, not one Unreal Data Layer or full copy
  per year. Data Layers may realize a small number of materially different authored phases.
- Define urbanization (“cityfication”) through a pinned metric or classification using population,
  built extent, density, functions, infrastructure, and land use. Do not invent one universal
  `is_city` threshold. Legal city status may be a separate political claim.
- Roads and structures crossing cells need one owner plus coverage/halo indexes. Interactive reads
  must not assemble a city from thousands of child cells.
- Building footprints, dimensions, material constraints, condition, and function are semantic;
  façade variants, props, road tessellation, traffic, HLOD, and skyline are deterministic
  presentation.

Start with one Pompeii phase transition and carry it through typed state, provenance, release
compile, snapshot, schema, and a presentation adapter before attempting a generic city generator.

## Contributing population, migration, and people

The `Population` layer is declared but not implemented. Canonical state should begin with bounded,
conserved aggregates and cohorts: estimated counts/ranges, demographics, household/economic roles,
settlement linkage, births/deaths, migration, displacement, and flows under a named model.

- Never run a persistent global simulation of every person. Planet/region views use precomputed
  aggregates and flows; only the observation bubble realizes bounded cohorts/individuals.
- Zooming in samples synthetic people deterministically from conserved cohorts. Zooming out destroys
  render entities and reabsorbs them without changing population totals.
- Synthetic names, appearances, routines, conversations, and exact locations are presentation unless
  a semantic event makes them noteworthy. They should not become permanent database identities.
- Historically attested people may have stable entities, but names, dates, roles, travel, kinship,
  and presence remain sourced claims with uncertainty and alternative hypotheses.
- Mass Entity is an optional active-bubble adapter, not canonical population storage. Every spawn,
  AI/task queue, path query, animation set, and crowd memory pool needs admission and a frame
  budget.

Test parent/child and cohort conservation, repeatable sampling, births/deaths/migration balance,
cross-cell flows, speed changes up to accelerated playback, zoom in/out reabsorption, cancellation,
and no growth in resident entities after repeated scrubbing.

## Contributing politics, borders, wars, and conflict

Use `Political` for polity identity, governance, control/ownership, borders, diplomacy, and
interpretations of political status. Use `Event` for wars, battles, sieges, raids, treaties,
revolts, and their uncertain participants/outcomes. Both layers are declared but not materialized
yet.

- Borders are uncertain, time-varying geometries/claims, not one precise modern polygon projected
  into the past. Distinguish claimed, administered, influenced, occupied, and disputed areas.
- A war does not imply that every city in a polity is actively fighting. Model theaters, fronts,
  routes, local battles, logistics, and city conditions separately so simultaneous distant life
  stays visible.
- Armies remain aggregate units/flows at broad scale. Only observed local units are realized, and
  exact unobserved formations/choreography are plausible or artistic presentation.
- Conflict consequences—deaths/displacement, burned vegetation, damaged structures, production/trade
  loss, or control changes—must be explicit typed outputs with provenance/model dependencies. An
  `Event` record alone cannot mutate unrelated state.
- Preserve competing chronologies, participant identities, casualty estimates, and territorial
  interpretations as hypotheses.

Test before/during/after boundaries, cities inside/outside the affected area, alternative
interpretations, conservation of troops/population/inventories, stable event identity, and bounded
local combat realization.

## Contributing eruptions, disasters, and devastation

Record the causal episode in `Event`; put persistent consequences in the affected semantic layers:

- deposits, crater/topography, subsidence, or landslide in geology;
- flood/river/coast changes in hydrology;
- vegetation loss and succession in biome;
- damaged/abandoned/rebuilt structures and land use in settlement;
- casualties, evacuation, displacement, and return in population; and
- emergency control or political changes in political state.

An eruption, earthquake, fire, flood, epidemic, or devastation period needs an uncertain temporal
envelope, affected geometry, intensity/model, causal dependencies, and pre/during/post segments.
“Devastated” should normally be a collection of typed state transitions and recovery trajectories,
not a permanent boolean or a destroyed mesh.

Ash, smoke, flame, collapse, debris, sound, screen effects, and cinematic timing are presentation
driven by the event seed and local semantic constraints. They may never feed nondeterministic
physics results back into canonical state. Vesuvius is the first test case, but replace the
synthetic fixture with licensed/cited inputs only through the future ingestion/review pipeline.

## Contributing exploration, “discovery,” and new land

Never add a global `discovered` flag. Separate three different concepts:

1. **Physical land emergence/exposure** is geology/hydrology, independent of human awareness.
2. **Travel, contact, mapping, or first attested arrival** is an event with participants, route,
   viewpoint, date/place uncertainty, and evidence.
3. **What a person, polity, or knowledge tradition knew** is perspective-specific claim/projection
   state. One society learning about a place does not reveal it to everyone or create the land.

Expeditions and encounters belong in `Event`, outposts in `Settlement`, and sovereignty claims in
`Political`. A historical map is an immutable source revision with its original frame/distortion and
evidence area. Repository discovery of new archaeological evidence uses transaction time, not the
represented historical timeline. “Fog of war” or map reveal is presentation of a selected observer's
knowledge, not world truth.

Avoid Eurocentric “first discovery” defaults. Name the observer/community and the exact definition
of first contact/record/arrival, retain indigenous and conflicting accounts, and expose uncertainty.

## Contributing economy, food, production, and trade

These are cross-domain models, not a reason to hide an unbounded economy simulator inside the
server. Represent resources, production capacities, land use, storage/inventories, demand, routes,
markets, and flows as typed, unit-bearing, uncertain aggregates linked to settlements and
populations.

- Define conservation/loss rules and temporal resolution for each commodity family.
- Store a route/network feature once; compile sparse flows/checkpoints rather than one row per
  traveler or item.
- Pin yield, climate, technology, transport, and behavioral model inputs.
- Keep coins, carts, market crowds, shop inventories, and ambient transactions as bounded
  realizations unless directly evidenced or semantically important.

Test conservation, route capacity, missing inputs, shocks from climate/conflict/disaster,
aggregation, and deterministic sampling at each detail level.

## Historical evidence and content contributions

Historical content should eventually enter as immutable source/citation/claim revisions inside a
reviewed change set, never as a direct edit to rendered world state. Every assertion needs:

- stable subject identity and versioned predicate schema;
- exact source revision/content hash and precise citation locator;
- creator/publisher, license/rights, attribution, and any access/redaction constraints;
- original date expression/calendar, uncertainty bounds, conversion model, and precision;
- spatial role (`findspot`, `origin`, `historical_footprint`, `path`, or `evidence_area`), raw
  coordinates/reference frame, accuracy, and possible envelope;
- explicit epistemic class and confidence assessment method; and
- reconstruction/model rationale plus exact input/output/build hashes for derived values.

Unknown is never rounded into an exact value. Contradictory admissible claims coexist as hypotheses;
moderation means admissible, not unquestionably true. Votes help discovery/review and never rewrite
a published release.

The project license and third-party content policy are not selected. Do not add externally licensed
historical data, maps, scans, meshes, textures, audio, or other assets until the rights/attribution
pipeline and repository license are decided.

## Using HistoryWorld in Unreal Engine

### What is checked in

The project is `unreal/HistoryWorldClient/HistoryWorldClient.uproject`, associated with UE 5.8. It
contains `HistoryWorldClient` and `HistoryWorldClientEditor` targets. The runtime integration is a
project-local beta plugin at:

```text
unreal/HistoryWorldClient/Plugins/HistoryWorldRuntime/
```

`HistoryWorldRuntime.uplugin` is source-only (`CanContainContent: false`), not an
installed/Marketplace plugin, and is already enabled in the checked-in `.uproject`. It currently
provides:

- reflected snapshot/cell/detail DTOs;
- `UHistoryWorldSubsystem`, a one-active-request `UGameInstanceSubsystem`;
- bounded response, identity, known-field, interval, and resource validation for the currently
  reflected snapshot subset;
- generation-based stale-request cancellation;
- `UHistoryWorldResourceSettings`; and
- `IHistoryWorldPresentationAdapter`, the seam for project-specific rendering.

It does not provide terrain, a map, Cesium, World Partition setup, PCG graphs, Mass crowds, 3D
assets, or an implementation of the presentation adapter.

The decoder is not yet a complete fail-closed mirror of the wire contract: it tolerates unknown JSON
fields, and `FHistoryWorldAppliedPatch` retains only the patch ID and epistemic class while dropping
the wire claim/evidence arrays. Add reflected references plus bounded decoding/validation before
calling the Unreal provenance UI complete.

### Open the checked-in project

1. Install a compatible UE 5.8 toolchain. The `.uproject` pins only the association; production also
   needs an exact engine build/commit, compiler/SDK, platform/RHI, plugin digests, renderer profile,
   semantic versions, and DDC namespace.
2. Start the service with `make run-server`.
3. Open `unreal/HistoryWorldClient/HistoryWorldClient.uproject` and build the prompted C++ modules.
4. In Project Settings, find **HistoryWorld Resource Budgets**, or edit
   `unreal/HistoryWorldClient/Config/DefaultEngine.ini`. The default service is
   `http://127.0.0.1:8080`.

   The current subsystem enforces only `ServiceBaseUrl`, `MaxHttpResponseBytes`,
   `MaxCandidatePatches`, `MaxWorkUnits`, `MaxGeneratedInstances`, and `RequestTimeoutSeconds`.
   `MaxResidentSemanticMegabytes`, `ProceduralFrameBudgetMilliseconds`, `MaxMassEntities`, and
   `MaxConcurrentRegionRequests` are future adapter/streamer settings and currently do not govern
   anything.

5. Resolve the current release outside the plugin:

   ```bash
   curl -s http://127.0.0.1:8080/v1/releases/current
   ```

6. In C++ or Blueprint, get the `HistoryWorldSubsystem` from the Game Instance, bind
   `OnSnapshotReady` and `OnSnapshotFailed`, and call `RequestSnapshot` with:

   - the returned 64-character lowercase release digest;
   - a valid `FHistoryWorldCell` (`Face`, `Level`, and hexadecimal `PathHex`);
   - canonical signed seconds from the HistoryWorld epoch; and
   - a requested detail from planet through street.

   For the current fixture, cell `0:14:ee085cd` is `Face=0`, `Level=14`, `PathHex=ee085cd`.
   `79-08-23 00:00:00` in the pinned clock is `AtSeconds=-59653929600`.

7. Apply a successful snapshot through a project-owned implementation of
   `IHistoryWorldPresentationAdapter`. Every asynchronous child task must retain/check the request
   generation and discard stale work.

These are local-toolchain templates, not CI commands. Replace the engine path with the exact pinned
UE 5.8 installation.

Linux:

```bash
export HW_UE_ROOT=/absolute/path/to/UE_5.8
export HW_PROJECT_FILE="$PWD/unreal/HistoryWorldClient/HistoryWorldClient.uproject"
"$HW_UE_ROOT/Engine/Build/BatchFiles/Linux/Build.sh" \
  HistoryWorldClientEditor Linux Development \
  -Project="$HW_PROJECT_FILE" -WaitMutex
"$HW_UE_ROOT/Engine/Binaries/Linux/UnrealEditor" "$HW_PROJECT_FILE"
```

macOS:

```bash
export HW_UE_ROOT=/absolute/path/to/UE_5.8
export HW_PROJECT_FILE="$PWD/unreal/HistoryWorldClient/HistoryWorldClient.uproject"
"$HW_UE_ROOT/Engine/Build/BatchFiles/Mac/Build.sh" \
  HistoryWorldClientEditor Mac Development \
  -Project="$HW_PROJECT_FILE" -WaitMutex
"$HW_UE_ROOT/Engine/Binaries/Mac/UnrealEditor.app/Contents/MacOS/UnrealEditor" \
  "$HW_PROJECT_FILE"
```

Windows Command Prompt:

```bat
set "HW_UE_ROOT=C:\absolute\path\to\UE_5.8"
set "HW_PROJECT_FILE=%CD%\unreal\HistoryWorldClient\HistoryWorldClient.uproject"
call "%HW_UE_ROOT%\Engine\Build\BatchFiles\Build.bat" ^
  HistoryWorldClientEditor Win64 Development ^
  -Project="%HW_PROJECT_FILE%" -WaitMutex
"%HW_UE_ROOT%\Engine\Binaries\Win64\UnrealEditor.exe" "%HW_PROJECT_FILE%"
```

Build `HistoryWorldClient` as well as `HistoryWorldClientEditor` before handing off engine changes.
Exact build/cook/package invocations belong in the future pinned engine lane once its platforms and
release profile are chosen.

The current subsystem requests all layers, nanoseconds zero, and one focused cell. A new request
cancels the old one. Do not create one subsystem per visible cell. The future globe streamer should
own a bounded priority queue keyed by release/cell/layers/detail, coalesce identical work, cache by
stable time segment and realization epoch, and enforce bounded concurrency/residency.

The current HTTP path buffers the complete response, converts it to UTF-16, and builds a JSON DOM on
the game thread. Blueprint multicast delegates also copy reflected arrays. Keep prototype responses
small. Before regional streaming, move bounded decode/validation to worker tasks, publish immutable
native handles, and apply work incrementally under the game/render-thread budget.

### Install the runtime plugin in another Unreal project

There is no Marketplace install step. Prefer a project-local copy so engine/project locks remain
reproducible:

1. Copy the complete `HistoryWorldRuntime` directory into
   `<YourProject>/Plugins/HistoryWorldRuntime/`.
2. Enable `HistoryWorldRuntime` in the Plugins UI or add the same enabled plugin entry to the target
   `.uproject`.
3. If a C++ module includes the plugin's public headers, add `HistoryWorldRuntime` to that module's
   `PublicDependencyModuleNames` or `PrivateDependencyModuleNames` as appropriate.
4. Regenerate project files if the platform workflow requires it, then build the Editor/Game target
   with the pinned engine toolchain.
5. Copy/configure the `[/Script/HistoryWorldRuntime.HistoryWorldResourceSettings]` section rather
   than hard-coding a service URL or budgets in presentation logic.

Do not put content in this plugin; its descriptor deliberately has `CanContainContent: false`. Put
Cesium, PCG, Mass, terrain, lighting, or other engine-specific dependencies behind a separate
project-owned presentation adapter/module. Pin every external plugin in the engine lock.

### Unreal validation and engine upgrades

The repository has no executable UBT/UHT/editor/cook/package/test command and CI has no proprietary
engine. `make ci` therefore does **not** prove Unreal compilation. When Unreal changes, run:

```bash
make format-unreal
make format-check-unreal
```

Then use the actual installed UE toolchain to build both Game and Editor targets and test load,
cook, and package. Record the exact invocation and result in the handoff. Keep `*.generated.h` last.

Before production cooking, turn `unreal/engine-lock.example.json` into a real pipeline lock and
replace every placeholder. Validate semantic golden hashes, response compatibility, all
cell/coordinate edge cases, rapid timeline scrubbing and cancellation, fly/load/unload soak
behavior, memory leaks, screenshots within tolerances, and per-platform CPU/GPU/RAM/VRAM/network
budgets.

Upgrade Unreal on an `engine-next` copy/branch because conversions are one-way. Rebuild presentation
artifacts from the same semantic fixtures and compare them; do not migrate historical truth into new
`.uasset` state. A future lighting, terrain, crowd, or rendering system changes the renderer profile
and derived cache key, not the corpus release.

## Contributing 3D assets

There is no production asset ingestion, Git LFS, attribution manifest, CAS publisher, or numerical
mesh/texture profile yet. Until the repository/content license and asset manifest plus archival,
LFS, or CAS policy are approved, do not commit any binary 3D, texture, or audio asset—including
self-authored assets. Contribute metadata/schema/tooling or an asset-manifest proposal only. First
add the agreed asset profile and archival workflow, with an ADR if it changes durable ownership.

Treat an asset as four different objects:

1. **Semantic feature:** what it is, where/when it exists, physical dimensions, footprint, heading,
   condition, function, and evidence/uncertainty.
2. **Immutable source asset:** scan, photogrammetry, excavation model, drawing, texture source, or
   editable artist master, stored by content hash with complete rights/provenance.
3. **Renderer-neutral derivative:** for example glTF/GLB or 3D Tiles, keyed by inputs, conversion
   tool, build, and asset/renderer profile.
4. **Unreal presentation cache:** imported `.uasset`, materials, HLOD, collision/nav proxy, Nanite,
   lighting, PCG/Mass output, DDC, and cook/package output.

The future asset manifest must include creator/source, license and redistribution/modification
rights, required attribution, all subasset dependencies, original and normalized hashes, tool/export
versions, epistemic class/evidence links, semantic feature/release IDs, units, handedness,
up/forward axes, conversion factor, pivot/anchor, bounding box, heading, and transformation steps.

Asset rules:

- Keep dimensions in fixed-point SI semantic data and georeference in canonical geometry. The
  adapter converts to Unreal centimetres, a local tangent scene, and rebased origins.
- Never hide latitude/longitude, elevation datum, historical orientation, existence dates, damage
  state, or building function only in mesh transforms or a level.
- Preserve source-quality originals outside Unreal. A `.uasset` must never be the sole or best copy.
- Use a versioned import/build recipe. Record color space, UV sets, normals/tangents, texture
  channel packing, texel density, material slots, LOD generation, collision, and nav policy.
- Prefer reusable modules, instances/material instances, shared textures/trim sheets, and
  deterministic variants. Do not give distant/decorative content expensive collision, ticks, unique
  materials, or heavyweight actors.
- LODs must reduce cost monotonically and preserve silhouette within a declared screen-error target.
  Nanite may be one profile, never the only compatibility path.
- Asset choice/variation uses an explicit realization seed and renderer profile. Filenames and
  random editor placement are not semantic identity.

Do not invent a universal triangle or texture limit. Add a versioned profile with measured ceilings
for source/derived bytes, download size, decompressed RAM, vertices/triangles per LOD, material
slots, draw calls, texture dimensions/mips, GPU upload and VRAM, collision complexity, generated
instances, game/render-thread application time, and unload behavior. Different
mobile/desktop/cinematic profiles may produce different visuals from the same semantic state.

Validate manifest/schema and dependency closure; rights/attribution; decodability; finite
coordinates, bounds, axes, pivot and scale; normals/tangents/UVs/materials; LOD monotonicity;
collision/nav; georeferencing, seams, poles/antimeridian and repeated origin shifts; pinned-engine
import/load/cook; screenshot tolerances; unload/leaks; and all CPU/RAM/GPU/VRAM/network/storage
gates.

The source-only `HistoryWorldRuntime` plugin cannot hold assets. When the policy exists, put a
coherent asset collection in a separate content plugin (`CanContainContent: true`) with a versioned
manifest and a presentation adapter/build recipe. Do not commit Unreal `Binaries`, `Intermediate`,
`Saved`, `DerivedDataCache`, `Releases`, `Content/Developers`, cooked packages, lighting builds, or
generated HLODs. Current `.gitattributes` marks binary extensions only; it is not Git LFS.

## Changing the deterministic inner core

Treat the core as a versioned compiler, not ordinary application code. Before changing any canonical
type, ordering, hash grammar, interval/cell rule, layer/detail ordinal, operation tag, numeric
model, PRNG, or snapshot meaning, identify the compatibility boundary. Such a change needs an ADR or
explicit migration story, the appropriate version/schema bump, and reviewed compatibility goldens.
Never silently reinterpret an existing byte or value.

### Time

`WorldInstant` is signed `i64` seconds from `1970-01-01T00:00:00` plus nanoseconds less than one
billion. Calendar helpers use proleptic Gregorian dates and astronomical years (`0` is 1 BCE). JSON
seconds are decimal strings so JavaScript and Unreal cannot lose precision. `TimeRange` is
start-inclusive/end-exclusive and an absent end is unbounded. Negative bucketing uses Euclidean
floor.

Do not replace this with PostgreSQL timestamps, JavaScript numbers, Unreal date/time, or one signed
nanosecond integer. Authoring also retains original date expression/calendar, conversion model,
precision, uncertainty bounds, and optional distribution. A clock/epoch/calendar/bucket change
propagates through clock version, hashes, realization epochs/stable segments/seeds, SQL checks,
OpenAPI, Proto, Unreal, fixtures, and boundary tests.

Test astronomical year zero, geological negative years, i64 extremes/overflow, nanos, invalid dates,
exact interval boundaries, uncertainty modes, unbounded ranges, and negative buckets.

### Cells and spatial references

`GeoPointE7` validates/canonicalizes portable coordinates; `SpatialReference` supplies
reference-frame meaning; `WorldCell` supplies a replaceable partition index. Preserve the v1 packed
layout, text format, path validation, and root-to-leaf ancestry semantics. Never change
`hw-cube-sphere-v1` in place or make its current floating projection a cross-platform runtime
authority.

Test every face, face/corner ties, poles, antimeridian, level 28, parent/child/raw/text round trips,
and cross-target offline goldens. Retain original geometry so future cell schemes can be rebuilt.

### IDs, hashes, ordering, and random streams

`StableId` is a 128-bit identity, not mutable facts. `ContentHash` is BLAKE3-256. Neither is a JSON
number. Canonical release and patch-content hashes use the manual, length-delimited binary grammar
in `historyworld-content`, not `serde_json` or incidental Protobuf bytes. `StableId` values are
encoded into that grammar as 16 big-endian bytes.

The compiler sorts provenance by total keys and patches by `(cell.raw(), canonical_order, id)`.
Runtime applies root-to-leaf, layer order, then canonical patch order. Hash maps, locale-sensitive
ordering, JSON serializer details, and thread completion order cannot enter identity.

Keep these identities separate:

- `release_id`: the complete semantic release;
- `local_dependency_hash`: selected local active dependencies plus pinned compiler/clock/cell
  contracts;
- `semantic_state_hash`: local dependencies plus quantized semantic state/provenance; and
- `realization_seed`: local dependencies plus materializer/numeric/PRNG versions, cell, layers,
  effective detail, epoch, namespace, and ordinal.

An unrelated global edit must not reshuffle an untouched place/time. Use independent seed
namespaces; never a global mutable RNG. Do not reuse operation, epistemic, evidence, layer/detail,
or Proto tags. `LayerKind` ordinals are mask bits and `DetailLevel` order affects downgrade/seeds.

### Content compiler

Compilation is the trust boundary from authored input to runtime state. Keep input fail-closed,
reject unknown fields and duplicate IDs, validate metadata/provenance bounds, normalize total
ordering, check all arithmetic/ranges, and reject releases with any predictable invalid transition.

Current ceilings include 65,536 patches/release, 64 claims and 64 evidence references/patch, 4,096
ancestry candidates/cell, 10,000,000 compiler-validation work units, and bounded metadata strings.
`validate_runtime_totality` proves stateful operations across all relevant validity boundaries.
Extend the proof and work accounting whenever operations become cross-field/layer/cell or depend on
prior state. Do not leave a release that validates offline but fails for a legal runtime query.

Adding a state field/operation requires domain validation, operation/layer/apply logic, a new
explicit canonical tag/byte encoding, totality proof, materializer hash/output accounting, schemas,
server behavior, fixtures/goldens, authoring projection, and Unreal DTO/range decoder if exposed.

### Materializer

`snapshot()` evaluates only requested layers along one cell ancestry. Admission must happen before
expensive scans, cloning, allocations, or application: candidate lower bound, deterministic work,
provenance cardinality/bytes, conservative output bytes, then exact encoded bytes at the HTTP edge.

All state application stays fixed-point, checked, and ordered. `stable_time_segment` is bounded by
the nearest selected-layer change. Spatial resolution, temporal resolution, semantic realization,
and visual detail stay independent; a detail downgrade changes realization only, not surface state.

Represent every new scan/allocation in work/output estimates. Parallelize only independent canonical
partitions with a fixed merge order. At high playback speeds sample states/transitions rather than
simulating skipped presentation frames.

Regression coverage should prove repeatability, input-order independence, root/child precedence,
half-open boundaries, overflow/missing-base rejection, unrelated/future-patch local reuse,
detail-downgrade semantic equality, independent seed namespaces, early limit rejection, and worker
count independence. Never update the Vesuvius golden hashes blindly; state why the canonical
identity changed and bump the right pre-release version/contract.

### Storage and publication

CAS objects are exact bytes addressed by BLAKE3, not semantic release IDs. Preserve bounded actual
reads, hash verification, symlink/non-regular-file rejection, no-overwrite atomic publication,
same-directory temporary files, concurrent-writer validation, and declared durability. Never trust a
metadata size check alone or overwrite a digest path.

Production publication must verify complete evidence/input dependency closure before moving a head.
Garbage collection is reachability-aware and retains published releases plus licensed/audit
material. Test corruption, size limits, concurrent writers/crashes, symlinks, durability, and
deduplication.

### HTTP service and resource admission

Client limits may only lower server ceilings. Preserve independent patch, deterministic-work,
response-byte, instance, wall-time, and compute/response concurrency controls. Blocking
materialization and JSON encoding run behind bounded admission; health/metrics remain available. A
slow client's body must retain bounded residency, and an edge proxy still needs
connection/header/idle-write/total-request timeouts.

The mutable current head is `no-cache`; pinned manifests are immutable with ETags; computed
snapshots are `private, no-cache` until the URL/artifact identity includes every
materialization/profile input. Never mark a representation immutable when code/config can change its
body for the same key. Errors are non-storable. Keep query rejection, status/JSON errors, ETags,
counters, and OpenAPI aligned.

Do not add unbounded caches/singleflight, per-viewer compression, or synchronous heavy work on async
I/O threads. Cache by bytes with complete keys and negative-entry limits; serve precompressed
immutable objects from object storage/CDN at scale.

### Wire contracts

OpenAPI describes JSON/HTTP; Proto is the additive compact-binary evolution path. Keep both
synchronized with Rust and Unreal. Deep-time seconds, IDs, hashes, and cells remain strings where
numeric precision would be unsafe. Optional values retain null/presence. Document/test the exact
`LayerMask` bit mapping.

Never change a Proto field's number/type/meaning; reserve deleted names/numbers and retain zero
`UNSPECIFIED` enum values. A breaking JSON meaning gets a new contract/API/schema. Add malformed and
compatibility fixtures for lengths, enums, required identity, canonical decimals, null-vs-zero, and
old/new consumer behavior.

### PostgreSQL/PostGIS authoring

World time is `(bigint seconds, integer nanos)`; `timestamptz` is only modern audit metadata.
`system_seq` is allocation order, not commit order, so exact hashed corpus membership is
authoritative and the sequence cutoff is only an audit watermark. Large binaries/chunks belong in
CAS, not rows or millions of partitions.

Migrations are append-only records: never edit or bulk-format a shipped migration; add the next
numbered file. Add append-only enforcement to new canonical tables, reverse-side indexes for foreign
keys/provenance traversal, and live PG15+PostGIS empty-database plus upgrade tests.

Do not assume current triggers seal publications or prove every graph/spatial invariant. Trusted
ingestion must still enforce revision-parent claim/acyclic rules, predicate/schema agreement,
SRID/reference-frame normalization, canonical hashes, restricted roles, and sealed/signed
publication.

## Core change propagation checklist

For any canonical/core change, search outward before editing:

1. Domain representation, validated constructors/serde, units, ordinals, and invariants.
2. Content normalization/order, hash bytes/tags, version constants, limits, and totality proof.
3. Materializer apply order, stable segment, dependency/state/seed hashes, work/output accounting.
4. CLI input/publication and CAS object/dependency identities.
5. Server parsing, clamp/admission, compute/body permit lifetime, errors, caching, ETags, and
   metrics.
6. OpenAPI and Protobuf compatibility contracts.
7. SQL trusted ingestion/projection and a new append-only migration.
8. Unreal DTO, decoder, request identity, generation cancellation, budgets, adapter/profile, engine
   lock, and fixture.
9. ADR/docs/migration story, focused tests, malformed tests, compatibility goldens, and benchmark
   resource statement.
10. `make ci`, plus separate Unreal build/cook and live PG15+PostGIS validation when applicable.

Redesign before merging if a proposal introduces a global tick, dense cell-by-time storage, a
runtime scan proportional to all history, interactive aggregation of thousands of children,
camera-dependent semantics, authoritative float/GPU output, shared RNG, unordered reductions,
unbounded queues/caches, cloning before admission, entry-count-only cache limits, per-viewer
recompression, or renderer artifacts as the only historical record.

## Definition of done

Before handing off a change, verify the applicable items:

- The change lives in the lowest correct layer and respects dependency direction.
- Evidence, uncertainty, epistemic class, rights, and reconstruction/model inputs are explicit.
- Units, ranges, null/unknown semantics, stable identity, time interval, space/reference frame, and
  aggregation rules are documented and validated.
- Canonical compatibility either stayed byte-for-byte stable or received an intentional ADR/version,
  migration/regeneration story, and reviewed golden changes.
- CPU/work, allocations/RAM, response/network bytes, storage growth, and GPU/VRAM/frame cost are
  finite and measured or bounded.
- Every new loop/queue/task/cache has admission, limits, cancellation/backpressure, complete keys,
  invalidation, and useful metrics.
- Focused tests cover happy paths, malformed/boundary inputs, determinism, and resource rejection.
- `make format` and `make ci` pass.
- Unreal C++ formatting and real UE build/load/cook/package tests ran when applicable, or the
  missing engine/toolchain is reported explicitly.
- A live PostgreSQL 15+PostGIS migration test ran when SQL changed, or that missing lane is
  reported.
- No generated caches, secrets, local data, externally unapproved assets, or unrelated user changes
  were added.
- Documentation and this guide were updated when the repository's operating contract changed.

Favor a small, evidence-traceable, deterministic, resource-bounded vertical slice over a broad
placeholder framework. That is how this project can keep improving indefinitely without its history,
performance, or future engine compatibility collapsing under its own scale.
