# Roadmap

The order follows dependency and evidence, not visual spectacle.

## Foundation — now

- Deep-time, globe, provenance, layer, and resource contracts.
- Deterministic content-addressed release compiler and materializer.
- Synthetic Vesuvius before/after compatibility fixture.
- Bounded CLI/HTTP read path and current-engine Unreal adapter seam.
- Verified filesystem content-addressed store with an S3-compatible adapter seam.
- Append-only PostgreSQL/PostGIS authoring schema.
- OpenAPI/Protobuf compatibility contracts, tests, and benchmark.

## Geological vertical slice

1. Choose licensed elevation/bathymetry and Vesuvius archaeology/geology sources.
2. Build source ingestion into a filesystem/S3-compatible content-addressed store; never embed large
   blobs in PostgreSQL.
3. Implement uncertain temporal extents and typed terrain projections.
4. Compile parent aggregate plus local pre/post-79 CE terrain cells with seam tests and provenance
   closure.
5. Publish immutable compressed chunks and exact dependency manifests.
6. Render globe-to-local transition in Unreal through Cesium plus one local World Partition scene,
   with collision/nav proxies and coordinate golden tests.

## Environment layers

- Paleogeography/plate-model adapters and reference-frame registry.
- Sea level, hydrology, climate envelopes, soils, and biome succession.
- Independent layer cadences, uncertainty visualization, and alternative interpretation switching.
- PCG recipes driven by semantic constraints, with baked coarse results and bounded cosmetic
  refinement.

## Human settlement slice

- Archaeological features, structures, routes, land use, production, trade, and population estimates
  as typed projections.
- Deterministic aggregate/cohort conservation across zoom levels.
- Pompeii 60/78/79/100 CE phases; timeline transitions and source inspector.
- Local Mass observation bubble; no persistent synthetic pedestrian stream.

## Collaborative encyclopedia

- Identity, change-set review, domain expertise, licensing/attribution closure, abuse controls, and
  auditable moderation projections.
- Conflicting hypothesis UI and possible/certain/preferred queries.
- Dependency-directed invalidation and reproducible release publishing.
- Web contributor/admin client; Unreal remains primarily the experiential view.

## Planetary scale

- Object-store/CDN delivery, content bundle mirrors, offline packs, and archive replication.
- Distributed offline compilation by canonical layer/cell/time partitions.
- Popular-region prebuilds, rare-view admission control, and trace-driven budgets.
- `engine-next` automation for every supported Unreal/plugin/platform lane.

Do not add global agents, Kubernetes, Kafka, Redis, a graph database, or a custom procedural DSL
until a measured slice demonstrates that the simpler modular monolith and immutable-object path
cannot meet a concrete requirement.
