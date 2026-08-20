# Meanwhile On Earth

_Explore history as a connected world, not a stack of disconnected dates._

## Why this project exists

History is usually taught one chapter, map, or civilization at a time. That makes it manageable, but
it hides some of the best questions. What was happening elsewhere at the same moment? How did a
river shape a settlement? What did a drought do to food, migration, and politics? How did an
eruption change a city before, during, and generations after it?

HistoryWorld exists to help students, teachers, researchers, and curious people see those
relationships. The hope is to give people a globe they can move through in place, time, and scale: a
way to ask not only “what happened here?” but also “what was connected to it?” and “how do we know?”

It is not meant to replace books, teachers, museums, fieldwork, or scholarship. It should be another
way into them—one that makes the wider world easier to hold in your head.

## The idea

Picture a globe with a time control. Pull back and watch coastlines, climate, vegetation,
settlement, trade, and political boundaries change. Move closer and see a region become roads,
farms, ports, and cities. At street level, a bounded part of the population can become people going
about their day. Pause, rewind, move somewhere else, and compare what was happening there at the
same time.

That experience must never pretend the past is known more precisely than it is. A measured
elevation, an ancient report, a scholarly inference, a model result, a plausible reconstruction, and
an invented background detail are different kinds of information. HistoryWorld is being built so
those differences remain visible, and so a viewer can eventually ask why something looks the way it
does.

There will not always be one accepted answer. Conflicting evidence and interpretations should be
able to coexist without one silently overwriting the others.

## Where the project is today

HistoryWorld is young. This repository contains the foundation beneath the experience, not a
finished historical world. There is no globe to fly around, no reconstructed Pompeii, and no
simulated city population yet.

What is working now is the part that would be painful to replace later:

- signed deep time, uncertain intervals, engine-neutral coordinates, and a versioned globe index;
- immutable semantic releases with provenance and deterministic hashes;
- fixed-point, on-demand snapshot materialization with hard work and byte limits;
- a verified local content-addressed store, a CLI, and a bounded read-only HTTP service;
- OpenAPI and Protobuf contracts plus an initial PostgreSQL/PostGIS authoring schema; and
- a small Unreal Engine 5.8 runtime plugin that consumes semantic snapshots without making Unreal
  assets the historical record.

Eight semantic layers are named in the contract: geology, hydrology, climate, biome, population,
settlement, political, and event. Only a few environmental surface values are materialized today:
elevation, relief, land and water fractions, temperature, precipitation, and vegetation fraction.
Cities, people, trade, politics, wars, eruptions, and other events are contribution lanes, not
finished systems.

The bundled Vesuvius example is a one-cell synthetic compatibility fixture. Every patch is labeled
`artistic_ambient`; it has no historical citations and is **not research or teaching material**. Its
job is to prove that the same inputs produce the same before-and-after states across an eruption
boundary.

## AI-assisted development, disclosed

There would be something odd about building a provenance-first history project while being vague
about the provenance of its own software.

HistoryWorld has been designed and coded with the help of AI coding assistants, and we expect to
keep using them. They have helped explore architecture, draft and refactor code, find edge cases,
write tests and documentation, and review changes. This README was also drafted with an AI assistant
under maintainer direction.

AI output is not treated as fact or authority. A model-generated statement is not historical
evidence, a plausible citation is not a real citation, and generated code is not correct merely
because it looks convincing. Whoever proposes or accepts a change remains responsible for checking
it. Code must survive review and automated checks; historical claims still need real sources; assets
still need clear rights and provenance.

Contributors are welcome to use AI assistants, but should disclose substantial AI assistance in the
change description, inspect the result, and run the same tests required of any other contribution.

## Help build it

This project is too broad for one person, one company, or one discipline. It needs people who know
how to read historical sources and people who know how to make a planet run smoothly—and people who
can help those groups understand one another.

There is useful work for:

- historians, archaeologists, archivists, and other source specialists;
- geologists, geographers, climate/ecology researchers, and GIS practitioners;
- teachers, students, museum workers, and accessibility researchers;
- Rust, database, distributed-systems, geospatial, and data-pipeline developers;
- Unreal Engine developers, graphics programmers, technical artists, and interaction designers;
- 3D pipeline, licensing, attribution, and digital-preservation specialists; and
- writers, reviewers, testers, security researchers, and people willing to question an assumption.

A contribution does not need to be grand. A careful test, a better explanation, a source-policy
proposal, a reproducible benchmark, or a well-described missing case can move the project forward.
The first geological slice, the asset/provenance pipeline, Unreal presentation adapters, source
review, and educator feedback all need help.

Please read [CONTRIBUTING.md](CONTRIBUTING.md) before preparing a change; the
[detailed technical guide](CLAUDE.md) is there when you need to work on a domain or the core. Start
with a short design issue when a decision affects canonical data, historical interpretation,
licensing, or long-term architecture. If this copy of the repository does not have a public issue
tracker yet, coordinate with the repository owner instead.

The project does not yet have a selected code/content license, asset manifest, or archival/LFS/CAS
policy for contributed media. Until those are settled, do not commit binary meshes, textures, audio,
or Unreal assets—even self-authored ones—and do not upload third-party historical data. The first 3D
asset contribution we need is the safe, attributable, reproducible pipeline that later assets can
use.

## How it holds together

One sentence guides the architecture: **Unreal is the stage, not the archive.**

Sources, claims, uncertainty, interpretations, and semantic state use engine-neutral formats. Unreal
turns that state into terrain, buildings, vegetation, crowds, lighting, sound, and interaction. A
future renderer or lighting system should improve the view by rebuilding disposable artifacts, not
by migrating the historical record.

```text
sources, measurements, maps, and scholarship
                       |
                       v
       immutable evidence and claim revisions
                       |
             reviewed corpus + interpretation
                       |
                       v
       deterministic, sparse semantic state
                       |
              versioned open contracts
                       |
                       v
       Unreal today, other clients tomorrow
```

The intended scaling model is equally important. A planetary view should read aggregates, not tick
every person on Earth. A regional view resolves sparse features and flows. A street view may
temporarily “crystallize” conserved population cohorts into a bounded crowd; zooming out removes the
rendered people without changing the population they represent.

There is no global simulation loop in the current server and no GPU work in canonical history. Work
is partitioned by semantic layer, globe cell, and stable time segment. Interactive requests carry
finite candidate-patch, work-unit, response-byte, and instance budgets; the service separately
bounds concurrency and wall time. The default snapshot budget is 4,096 candidate patches, 100,000
work units, an 8 MiB encoded response, and 20,000 suggested instances. See
[the performance budget](docs/performance-budgets.md) for the full resource policy.

The deeper architectural decisions are recorded rather than left as folklore:

- [Unreal is a replaceable presentation client](docs/adr/0001-engine-neutral-core.md)
- [Deep time and repository time are separate](docs/adr/0002-deep-time-and-bitemporality.md)
- [The globe index is versioned and replaceable](docs/adr/0003-versioned-globe-index.md)
- [Materialization is bounded and demand-driven](docs/adr/0004-bounded-demand-materialization.md)

## Try the foundation

Rust 1.91.1 is pinned through `rust-toolchain.toml`. Repository-wide checks also require Node 22 and
Go. `make setup` installs the pinned npm tools; it does not install Rust, Node, Go, Unreal Engine,
or ClangFormat.

```bash
make setup
make ci
make demo
```

To inspect the synthetic Vesuvius boundary directly:

```bash
cargo run --locked -p historyworld-cli -- validate
cargo run --locked -p historyworld-cli -- snapshot --at-year 79 --month 8 --day 23
cargo run --locked -p historyworld-cli -- snapshot --at-year 79 --month 8 --day 25
```

Repeated requests produce the same release ID, semantic state hash, provenance order, and
realization seed. The two dates fall on opposite sides of the fixture's synthetic eruption change.

Run the local service with:

```bash
make run-server
curl -s http://127.0.0.1:8080/healthz
curl -s http://127.0.0.1:8080/v1/releases/current
```

The service currently serves only the compiled-in fixture. It does not read PostgreSQL, local
published artifacts, or arbitrary releases, and it is not hardened for public internet exposure. The
local `publish` CLI command is a CAS demonstration; it does not prove production evidence or
dependency closure.

## Unreal Engine

The checked-in project targets Unreal Engine 5.8. `HistoryWorldRuntime` is an already-enabled,
source-only, project-local plugin—not a Marketplace plugin. It provides snapshot DTOs, a
focused-view request subsystem, cancellation, resource settings, and the presentation-adapter
interface. It does not yet provide terrain, a map, Cesium, World Partition setup, PCG, Mass, 3D
content, or an adapter implementation.

Start the service with `make run-server`, then open:

```text
unreal/HistoryWorldClient/HistoryWorldClient.uproject
```

To use the runtime module in another project, copy the whole
`unreal/HistoryWorldClient/Plugins/HistoryWorldRuntime` directory into that project's `Plugins`
directory, enable it, add the module dependency where its C++ headers are used, and rebuild with a
compatible UE 5.8 toolchain. Pin the exact engine build and toolchain before a production cook.

The current decoder and Blueprint-facing data are a small-snapshot prototype: unknown JSON fields
are tolerated, detailed claim/evidence references are not yet exposed, and response decoding builds
a JSON DOM on the game thread. Keep the payload small. The plugin boundary and known limitations are
in [the Unreal guide](unreal/README.md); installation, platform build templates, and settings are in
the [full repository guide](CLAUDE.md#using-historyworld-in-unreal-engine). `make ci` does not
compile Unreal.

## Repository map

```text
apps/                        CLI and bounded read service
crates/historyworld-domain  Deep time, globe cells, provenance, layers, budgets
crates/historyworld-content Immutable release compiler and semantic patches
crates/historyworld-sim     Deterministic on-demand materializer and benchmark
crates/historyworld-storage Verified filesystem content-addressed store
data/fixtures/               Synthetic compatibility/demo inputs
infra/postgres/              Append-only collaborative authoring schema
schemas/                     OpenAPI and versioned Protobuf contracts
unreal/                      UE 5.8 presentation project and runtime plugin
docs/                        Architecture, data model, budgets, roadmap, ADRs
```

For a guided tour, continue with:

- [Architecture](docs/architecture.md)
- [Data and provenance model](docs/data-model.md)
- [Performance and resource budgets](docs/performance-budgets.md)
- [Roadmap](docs/roadmap.md)
- [Contribution guide](CONTRIBUTING.md)
- [Detailed repository and domain guide](CLAUDE.md)

## License and content rights

HistoryWorld does not yet have a selected open-source or content license. The repository is
therefore not currently offering the usual open-source reuse rights. Please coordinate before
preparing a substantial contribution, and do not submit third-party datasets or media while the
licensing, attribution, and archival policies are unfinished.

Choosing those policies is part of building the project responsibly—not an afterthought.
