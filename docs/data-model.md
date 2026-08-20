# Data and provenance model

## Knowledge is not a mutable world row

The permanent pipeline is:

```text
source revision → citation → claim revision → moderation event
                → corpus release → interpretation revision
                → simulation/derivation manifest → semantic state chunk
```

Each arrow is queryable so the UI can answer “why does this look this way?” A visible value must
lead to evidence, an explicit model, a plausible reconstruction, or an artistic label.

Stable entities carry identity only. Names, locations, ownership, borders, population,
relationships, and “same as” are claims. Concurrent edits create sibling revisions; neither silently
wins. A merge revision names both parents.

Moderation means admissible into a corpus, not certain truth. Contradictory admissible hypotheses
may coexist, and an interpretation chooses among them. Votes and reputation are domain-specific
review/ranking signals; changing a vote does not rewrite a published release or invalidate world
caches.

## Two time axes

World-valid time answers “when in represented history?” Repository transaction time answers “what
did contributors know/accept at sequence N?”

The PostgreSQL migration stores world time as signed seconds/nanoseconds and orders repository
events with a monotonic `system_seq`. PostgreSQL `timestamp` is used only for modern audit metadata
because its year range cannot represent the project's prehistoric/geological scope.

Uncertain dating retains:

- earliest/latest possible beginning;
- earliest/latest possible end;
- optional preferred bounds;
- original expression and calendar;
- conversion model and precision;
- optional probability-distribution artifact.

A query must eventually choose possible, certain, preferred, or sampled semantics explicitly.
“Unknown” is never rounded into an exact date.

## Space and uncertainty

Findspot, manufacture origin, historical footprint, route, and evidence area are different spatial
roles. A spatial extent pins the reference frame, raw source coordinates, normalized geometry,
coordinate epoch, horizontal/vertical accuracy, possible envelope, and optional
reconstruction/probability assets.

Deep-time plate reconstruction is a versioned derivation. Changing the plate model creates a
different geometry/content hash; it does not mutate the source.

## Releases and hashes

A corpus release freezes exact claim revisions under a moderation cutoff and Merkle root. An
interpretation revision pins hypothesis choices and model parameters. A semantic knowledge release
additionally pins:

- world-clock and cell-scheme versions;
- canonical serialization;
- exact input hashes.

A materialization run/artifact then pins the semantic release plus the simulator/materializer build,
numeric model, PRNG algorithm, independent subsystem seeds, effective resource/profile inputs, and
exact dependency manifest. Keeping these identities separate lets unchanged local semantic chunks
survive an unrelated global corpus release without weakening replay.

No artifact refers to `latest`. Corrections append revisions and publish a new release. Old views
remain reproducible.

The initial Rust compiler canonicalizes patch order and provenance reference order, rejects
duplicate IDs, unknown fields, unsupported clock/cell/schema versions, and runtime-invalid
transition segments. Release IDs use an explicit length-prefixed binary hash grammar rather than
depending on a JSON library's byte output. The versioned Protobuf contract is the compact binary
evolution path.

## Storage roles

PostgreSQL/PostGIS contains collaboration workflow, lightweight geometries, manifests, indexes, and
typed hot projections. Object storage contains original assets, rasters, terrain, meshes,
event/state chunks, probability data, Parquet, and checkpoints. CDN serves immutable hot objects.

Do not create billions of event rows or millions of database partitions. Bulk events are sorted by
canonical key into layer/cell/time chunks. Features are stored once by stable ID and indexed into
covering cells rather than duplicated as canonical tile payloads.

Derived objects are garbage-collectable. Licensed raw sources and audit history are checksummed,
replicated, and subject only to explicit privacy/legal redaction workflows that leave tombstones.

## Foundation database boundaries

The initial migration targets PostgreSQL 15 or newer (`UNIQUE NULLS NOT DISTINCT`) with PostGIS. It
enforces nanosecond co-presence/ranges, preferred time bounds, one revision of a claim per corpus
release, evidence roles, hash widths, and append-only updates/deletes. Reverse foreign-key indexes
keep the main provenance and release traversals from degrading into table scans.

Several controls deliberately remain at the trusted ingestion/release-builder boundary until their
semantics are finalized:

- `system_seq` is allocation order, not PostgreSQL commit order. The exact, hashed corpus membership
  manifest is authoritative; a moderation cutoff is only an audit watermark.
- Append-only membership tables are not sealed merely because their parent row exists. Production
  publication needs restricted database roles plus an explicit signed/sealed release transition.
- Revision-parent edges still need same-claim and acyclic-graph validation, and claim predicate keys
  still need validation against the referenced predicate schema.
- PostGIS geometries still need ingestion-time SRID/reference-frame and dimensional normalization.
  The database does not derive canonical content hashes from JSON or geometry values.
- Table owners can bypass triggers and `TRUNCATE` is outside the row triggers; production roles and
  backups are part of the integrity model.
