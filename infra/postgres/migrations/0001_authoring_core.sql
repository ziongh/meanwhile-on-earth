BEGIN;

CREATE EXTENSION IF NOT EXISTS postgis;
CREATE SEQUENCE IF NOT EXISTS historyworld_system_seq AS bigint;

-- World-valid time uses signed seconds, never PostgreSQL timestamp. Modern
-- repository transaction time still uses timestamptz plus a total-order seq.
CREATE TABLE reference_frame (
  id uuid PRIMARY KEY,
  authority text NOT NULL,
  code text NOT NULL,
  revision text NOT NULL,
  ellipsoid text NOT NULL,
  vertical_datum text NOT NULL,
  coordinate_epoch_seconds bigint,
  coordinate_epoch_nanos integer,
  reconstruction_model_hash bytea,
  created_seq bigint NOT NULL DEFAULT nextval('historyworld_system_seq'),
  recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
  CHECK ((coordinate_epoch_seconds IS NULL) = (coordinate_epoch_nanos IS NULL)),
  CHECK (coordinate_epoch_nanos IS NULL OR coordinate_epoch_nanos BETWEEN 0 AND 999999999),
  CHECK (reconstruction_model_hash IS NULL OR octet_length(reconstruction_model_hash) = 32),
  UNIQUE NULLS NOT DISTINCT (authority, code, revision, vertical_datum, reconstruction_model_hash)
);

CREATE TABLE temporal_extent (
  id uuid PRIMARY KEY,
  begin_earliest_seconds bigint NOT NULL,
  begin_earliest_nanos integer NOT NULL DEFAULT 0,
  begin_latest_seconds bigint NOT NULL,
  begin_latest_nanos integer NOT NULL DEFAULT 0,
  end_earliest_seconds bigint,
  end_earliest_nanos integer,
  end_latest_seconds bigint,
  end_latest_nanos integer,
  preferred_begin_seconds bigint,
  preferred_begin_nanos integer,
  preferred_end_seconds bigint,
  preferred_end_nanos integer,
  source_expression text NOT NULL,
  source_calendar text NOT NULL,
  conversion_model_version text NOT NULL,
  precision_descriptor text NOT NULL,
  probability_asset_hash bytea,
  created_seq bigint NOT NULL DEFAULT nextval('historyworld_system_seq'),
  recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
  CHECK (begin_earliest_nanos BETWEEN 0 AND 999999999),
  CHECK (begin_latest_nanos BETWEEN 0 AND 999999999),
  CHECK (end_earliest_nanos IS NULL OR end_earliest_nanos BETWEEN 0 AND 999999999),
  CHECK (end_latest_nanos IS NULL OR end_latest_nanos BETWEEN 0 AND 999999999),
  CHECK (preferred_begin_nanos IS NULL OR preferred_begin_nanos BETWEEN 0 AND 999999999),
  CHECK (preferred_end_nanos IS NULL OR preferred_end_nanos BETWEEN 0 AND 999999999),
  CHECK ((begin_earliest_seconds, begin_earliest_nanos) <= (begin_latest_seconds, begin_latest_nanos)),
  CHECK ((end_earliest_seconds IS NULL) = (end_earliest_nanos IS NULL)),
  CHECK ((end_latest_seconds IS NULL) = (end_latest_nanos IS NULL)),
  CHECK ((end_earliest_seconds IS NULL) = (end_latest_seconds IS NULL)),
  CHECK (
    end_earliest_seconds IS NULL
    OR (end_earliest_seconds, end_earliest_nanos) <= (end_latest_seconds, end_latest_nanos)
  ),
  CHECK (
    end_latest_seconds IS NULL
    OR (begin_earliest_seconds, begin_earliest_nanos) < (end_latest_seconds, end_latest_nanos)
  ),
  CHECK ((preferred_begin_seconds IS NULL) = (preferred_begin_nanos IS NULL)),
  CHECK ((preferred_end_seconds IS NULL) = (preferred_end_nanos IS NULL)),
  CHECK (preferred_end_seconds IS NULL OR preferred_begin_seconds IS NOT NULL),
  CHECK (
    preferred_begin_seconds IS NULL
    OR (
      (preferred_begin_seconds, preferred_begin_nanos)
        BETWEEN (begin_earliest_seconds, begin_earliest_nanos)
        AND (begin_latest_seconds, begin_latest_nanos)
      AND (
        (
          end_earliest_seconds IS NULL
          AND preferred_end_seconds IS NULL
        )
        OR (
          end_earliest_seconds IS NOT NULL
          AND preferred_end_seconds IS NOT NULL
          AND (preferred_end_seconds, preferred_end_nanos)
            BETWEEN (end_earliest_seconds, end_earliest_nanos)
            AND (end_latest_seconds, end_latest_nanos)
          AND (preferred_begin_seconds, preferred_begin_nanos)
            < (preferred_end_seconds, preferred_end_nanos)
        )
      )
    )
  ),
  CHECK (probability_asset_hash IS NULL OR octet_length(probability_asset_hash) = 32)
);

CREATE INDEX temporal_extent_possible_time_idx
  ON temporal_extent (begin_earliest_seconds, end_latest_seconds);
CREATE INDEX temporal_extent_created_seq_idx ON temporal_extent (created_seq);

CREATE TABLE spatial_extent (
  id uuid PRIMARY KEY,
  role text NOT NULL CHECK (role IN ('findspot', 'historical_footprint', 'path', 'origin', 'evidence_area')),
  reference_frame_id uuid NOT NULL REFERENCES reference_frame(id),
  geometry geometry(GeometryZ) NOT NULL,
  raw_source_coordinates jsonb NOT NULL,
  observed_epoch_seconds bigint,
  observed_epoch_nanos integer,
  horizontal_accuracy_mm bigint,
  vertical_accuracy_mm bigint,
  possible_envelope geometry(GeometryZ),
  probability_asset_hash bytea,
  content_hash bytea NOT NULL CHECK (octet_length(content_hash) = 32),
  created_seq bigint NOT NULL DEFAULT nextval('historyworld_system_seq'),
  recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
  CHECK ((observed_epoch_seconds IS NULL) = (observed_epoch_nanos IS NULL)),
  CHECK (observed_epoch_nanos IS NULL OR observed_epoch_nanos BETWEEN 0 AND 999999999),
  CHECK (horizontal_accuracy_mm IS NULL OR horizontal_accuracy_mm >= 0),
  CHECK (vertical_accuracy_mm IS NULL OR vertical_accuracy_mm >= 0),
  CHECK (probability_asset_hash IS NULL OR octet_length(probability_asset_hash) = 32)
);

CREATE INDEX spatial_extent_geometry_gist ON spatial_extent USING gist (geometry);
CREATE INDEX spatial_extent_envelope_gist ON spatial_extent USING gist (possible_envelope);

CREATE TABLE source_document (
  id uuid PRIMARY KEY,
  source_type text NOT NULL,
  external_identifier text,
  rights_expression text NOT NULL,
  attribution text NOT NULL,
  canonical_metadata jsonb NOT NULL,
  created_seq bigint NOT NULL DEFAULT nextval('historyworld_system_seq'),
  recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp()
);

CREATE TABLE source_revision (
  id uuid PRIMARY KEY,
  source_id uuid NOT NULL REFERENCES source_document(id),
  immutable_asset_hash bytea,
  external_snapshot_hash bytea,
  bibliographic_revision jsonb NOT NULL,
  content_hash bytea NOT NULL UNIQUE CHECK (octet_length(content_hash) = 32),
  authored_by uuid NOT NULL,
  created_seq bigint NOT NULL DEFAULT nextval('historyworld_system_seq'),
  recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
  CHECK (immutable_asset_hash IS NULL OR octet_length(immutable_asset_hash) = 32),
  CHECK (external_snapshot_hash IS NULL OR octet_length(external_snapshot_hash) = 32)
);

CREATE TABLE entity (
  id uuid PRIMARY KEY,
  kind text NOT NULL,
  created_by uuid NOT NULL,
  created_seq bigint NOT NULL DEFAULT nextval('historyworld_system_seq'),
  recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp()
);

CREATE TABLE predicate_schema (
  id uuid PRIMARY KEY,
  predicate_key text NOT NULL,
  semantic_version text NOT NULL,
  json_schema_asset_hash bytea NOT NULL CHECK (octet_length(json_schema_asset_hash) = 32),
  projector_build_id text NOT NULL,
  created_seq bigint NOT NULL DEFAULT nextval('historyworld_system_seq'),
  recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
  UNIQUE (predicate_key, semantic_version)
);

CREATE TABLE hypothesis_set (
  id uuid PRIMARY KEY,
  completeness text NOT NULL CHECK (completeness IN ('open', 'exhaustive')),
  created_by uuid NOT NULL,
  created_seq bigint NOT NULL DEFAULT nextval('historyworld_system_seq'),
  recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp()
);

CREATE TABLE claim (
  id uuid PRIMARY KEY,
  subject_entity_id uuid NOT NULL REFERENCES entity(id),
  predicate_key text NOT NULL,
  created_by uuid NOT NULL,
  created_seq bigint NOT NULL DEFAULT nextval('historyworld_system_seq'),
  recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp()
);

CREATE TABLE hypothesis_member (
  hypothesis_set_id uuid NOT NULL REFERENCES hypothesis_set(id),
  claim_id uuid NOT NULL REFERENCES claim(id),
  PRIMARY KEY (hypothesis_set_id, claim_id)
);

CREATE TABLE claim_revision (
  id uuid PRIMARY KEY,
  claim_id uuid NOT NULL REFERENCES claim(id),
  revision_number integer NOT NULL CHECK (revision_number > 0),
  predicate_schema_id uuid NOT NULL REFERENCES predicate_schema(id),
  object_entity_id uuid REFERENCES entity(id),
  payload jsonb NOT NULL,
  temporal_extent_id uuid NOT NULL REFERENCES temporal_extent(id),
  epistemic_class text NOT NULL CHECK (
    epistemic_class IN (
      'direct_observation', 'reported_primary_source', 'scholarly_inference',
      'model_derived', 'plausible_reconstruction', 'artistic_ambient'
    )
  ),
  confidence_descriptor jsonb,
  authored_by uuid NOT NULL,
  content_hash bytea NOT NULL UNIQUE CHECK (octet_length(content_hash) = 32),
  authored_seq bigint NOT NULL DEFAULT nextval('historyworld_system_seq'),
  recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
  UNIQUE (claim_id, revision_number),
  UNIQUE (id, claim_id)
);

CREATE INDEX claim_revision_claim_idx ON claim_revision (claim_id, revision_number DESC);
CREATE INDEX claim_revision_authored_seq_idx ON claim_revision (authored_seq);

CREATE TABLE claim_revision_parent (
  child_revision_id uuid NOT NULL REFERENCES claim_revision(id),
  parent_revision_id uuid NOT NULL REFERENCES claim_revision(id),
  PRIMARY KEY (child_revision_id, parent_revision_id),
  CHECK (child_revision_id <> parent_revision_id)
);

CREATE TABLE claim_spatial_extent (
  claim_revision_id uuid NOT NULL REFERENCES claim_revision(id),
  spatial_extent_id uuid NOT NULL REFERENCES spatial_extent(id),
  PRIMARY KEY (claim_revision_id, spatial_extent_id)
);

CREATE TABLE citation (
  id uuid PRIMARY KEY,
  source_revision_id uuid NOT NULL REFERENCES source_revision(id),
  locator jsonb NOT NULL,
  excerpt_hash bytea,
  note text,
  content_hash bytea NOT NULL UNIQUE CHECK (octet_length(content_hash) = 32),
  created_by uuid NOT NULL,
  created_seq bigint NOT NULL DEFAULT nextval('historyworld_system_seq'),
  recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
  CHECK (excerpt_hash IS NULL OR octet_length(excerpt_hash) = 32)
);

CREATE TABLE claim_evidence (
  claim_revision_id uuid NOT NULL REFERENCES claim_revision(id),
  citation_id uuid NOT NULL REFERENCES citation(id),
  role text NOT NULL CHECK (role IN ('supports', 'contradicts', 'contextualizes')),
  strength_descriptor jsonb,
  note text,
  PRIMARY KEY (claim_revision_id, citation_id, role)
);

CREATE TABLE change_set (
  id uuid PRIMARY KEY,
  base_release_id bytea,
  author_id uuid NOT NULL,
  submitted_seq bigint NOT NULL DEFAULT nextval('historyworld_system_seq'),
  recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
  CHECK (base_release_id IS NULL OR octet_length(base_release_id) = 32)
);

CREATE TABLE change_set_item (
  change_set_id uuid NOT NULL REFERENCES change_set(id),
  claim_revision_id uuid NOT NULL REFERENCES claim_revision(id),
  PRIMARY KEY (change_set_id, claim_revision_id)
);

CREATE TABLE review_event (
  id uuid PRIMARY KEY,
  change_set_id uuid REFERENCES change_set(id),
  claim_revision_id uuid REFERENCES claim_revision(id),
  reviewer_id uuid NOT NULL,
  action text NOT NULL CHECK (action IN ('comment', 'request_changes', 'recommend_accept', 'recommend_reject')),
  rationale text NOT NULL,
  policy_version text NOT NULL,
  system_seq bigint NOT NULL DEFAULT nextval('historyworld_system_seq'),
  recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
  CHECK (change_set_id IS NOT NULL OR claim_revision_id IS NOT NULL)
);

CREATE TABLE moderation_event (
  id uuid PRIMARY KEY,
  claim_revision_id uuid NOT NULL REFERENCES claim_revision(id),
  action text NOT NULL CHECK (action IN ('admit', 'reject', 'deprecate', 'supersede', 'restore')),
  moderator_id uuid NOT NULL,
  rationale text NOT NULL,
  policy_version text NOT NULL,
  system_seq bigint NOT NULL DEFAULT nextval('historyworld_system_seq'),
  recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp()
);

CREATE INDEX moderation_event_target_seq_idx
  ON moderation_event (claim_revision_id, system_seq DESC);

CREATE TABLE vote_signal (
  id uuid PRIMARY KEY,
  actor_id uuid NOT NULL,
  target_revision_id uuid NOT NULL REFERENCES claim_revision(id),
  value smallint NOT NULL CHECK (value IN (-1, 1)),
  expertise_domain text,
  system_seq bigint NOT NULL DEFAULT nextval('historyworld_system_seq'),
  recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp()
);

CREATE TABLE corpus_release (
  id bytea PRIMARY KEY CHECK (octet_length(id) = 32),
  parent_release_id bytea REFERENCES corpus_release(id),
  moderation_cutoff_seq bigint NOT NULL,
  membership_manifest_asset_hash bytea NOT NULL CHECK (octet_length(membership_manifest_asset_hash) = 32),
  merkle_root bytea NOT NULL CHECK (octet_length(merkle_root) = 32),
  schema_registry_version text NOT NULL,
  created_by uuid NOT NULL,
  created_seq bigint NOT NULL DEFAULT nextval('historyworld_system_seq'),
  recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp()
);

CREATE TABLE corpus_release_claim (
  release_id bytea NOT NULL REFERENCES corpus_release(id),
  claim_revision_id uuid NOT NULL,
  -- Deliberately denormalized so the database can enforce one revision of a
  -- stable claim per release; the composite FK prevents drift.
  claim_id uuid NOT NULL,
  PRIMARY KEY (release_id, claim_revision_id),
  UNIQUE (release_id, claim_id),
  FOREIGN KEY (claim_revision_id, claim_id) REFERENCES claim_revision(id, claim_id)
);

CREATE TABLE interpretation_revision (
  id bytea PRIMARY KEY CHECK (octet_length(id) = 32),
  corpus_release_id bytea NOT NULL REFERENCES corpus_release(id),
  parent_revision_id bytea REFERENCES interpretation_revision(id),
  policy_asset_hash bytea NOT NULL CHECK (octet_length(policy_asset_hash) = 32),
  hypothesis_choices_asset_hash bytea NOT NULL CHECK (octet_length(hypothesis_choices_asset_hash) = 32),
  reconstruction_parameters_asset_hash bytea NOT NULL CHECK (octet_length(reconstruction_parameters_asset_hash) = 32),
  ruleset_version text NOT NULL,
  created_by uuid NOT NULL,
  created_seq bigint NOT NULL DEFAULT nextval('historyworld_system_seq'),
  recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp()
);

CREATE TABLE derivation_activity (
  id uuid PRIMARY KEY,
  activity_type text NOT NULL,
  actor_or_software_build text NOT NULL,
  parameters_asset_hash bytea NOT NULL CHECK (octet_length(parameters_asset_hash) = 32),
  started_at timestamptz NOT NULL,
  completed_at timestamptz,
  created_seq bigint NOT NULL DEFAULT nextval('historyworld_system_seq'),
  recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
  CHECK (completed_at IS NULL OR completed_at >= started_at)
);

CREATE TABLE derivation_input (
  activity_id uuid NOT NULL REFERENCES derivation_activity(id),
  input_kind text NOT NULL,
  input_id text NOT NULL,
  input_content_hash bytea NOT NULL CHECK (octet_length(input_content_hash) = 32),
  role text NOT NULL,
  PRIMARY KEY (activity_id, input_kind, input_id, role)
);

CREATE TABLE derivation_output (
  activity_id uuid NOT NULL REFERENCES derivation_activity(id),
  output_kind text NOT NULL,
  output_id text NOT NULL,
  output_content_hash bytea NOT NULL CHECK (octet_length(output_content_hash) = 32),
  role text NOT NULL,
  PRIMARY KEY (activity_id, output_kind, output_id, role)
);

CREATE TABLE simulation_run (
  id bytea PRIMARY KEY CHECK (octet_length(id) = 32),
  interpretation_revision_id bytea NOT NULL REFERENCES interpretation_revision(id),
  simulator_build_digest bytea NOT NULL CHECK (octet_length(simulator_build_digest) = 32),
  ruleset_version text NOT NULL,
  world_clock_version text NOT NULL,
  numeric_model_version text NOT NULL,
  prng_algorithm_version text NOT NULL,
  root_seed bytea NOT NULL CHECK (octet_length(root_seed) = 32),
  start_seconds bigint NOT NULL,
  start_nanos integer NOT NULL CHECK (start_nanos BETWEEN 0 AND 999999999),
  end_seconds bigint,
  end_nanos integer,
  resolution_policy_version text NOT NULL,
  run_manifest_hash bytea NOT NULL CHECK (octet_length(run_manifest_hash) = 32),
  created_seq bigint NOT NULL DEFAULT nextval('historyworld_system_seq'),
  recorded_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
  CHECK ((end_seconds IS NULL) = (end_nanos IS NULL)),
  CHECK (end_nanos IS NULL OR end_nanos BETWEEN 0 AND 999999999),
  CHECK (end_seconds IS NULL OR (start_seconds, start_nanos) < (end_seconds, end_nanos))
);

-- PostgreSQL does not automatically index the referencing side of foreign
-- keys. These indexes keep release construction, provenance traversal, and
-- deletion-safety checks bounded as the authoring corpus grows.
CREATE INDEX spatial_extent_reference_frame_idx
  ON spatial_extent (reference_frame_id);
CREATE INDEX source_revision_source_idx
  ON source_revision (source_id, created_seq DESC);
CREATE INDEX claim_subject_idx
  ON claim (subject_entity_id, predicate_key);
CREATE INDEX hypothesis_member_claim_idx
  ON hypothesis_member (claim_id, hypothesis_set_id);
CREATE INDEX claim_revision_predicate_schema_idx
  ON claim_revision (predicate_schema_id);
CREATE INDEX claim_revision_object_entity_idx
  ON claim_revision (object_entity_id) WHERE object_entity_id IS NOT NULL;
CREATE INDEX claim_revision_temporal_extent_idx
  ON claim_revision (temporal_extent_id);
CREATE INDEX claim_revision_parent_parent_idx
  ON claim_revision_parent (parent_revision_id, child_revision_id);
CREATE INDEX claim_spatial_extent_spatial_idx
  ON claim_spatial_extent (spatial_extent_id, claim_revision_id);
CREATE INDEX citation_source_revision_idx
  ON citation (source_revision_id);
CREATE INDEX claim_evidence_citation_idx
  ON claim_evidence (citation_id, claim_revision_id);
CREATE INDEX change_set_item_revision_idx
  ON change_set_item (claim_revision_id, change_set_id);
CREATE INDEX review_event_change_set_seq_idx
  ON review_event (change_set_id, system_seq DESC) WHERE change_set_id IS NOT NULL;
CREATE INDEX review_event_revision_seq_idx
  ON review_event (claim_revision_id, system_seq DESC) WHERE claim_revision_id IS NOT NULL;
CREATE INDEX vote_signal_target_seq_idx
  ON vote_signal (target_revision_id, system_seq DESC);
CREATE INDEX corpus_release_parent_idx
  ON corpus_release (parent_release_id) WHERE parent_release_id IS NOT NULL;
CREATE INDEX corpus_release_claim_revision_idx
  ON corpus_release_claim (claim_revision_id, release_id);
CREATE INDEX corpus_release_claim_claim_idx
  ON corpus_release_claim (claim_id, release_id);
CREATE INDEX interpretation_revision_corpus_idx
  ON interpretation_revision (corpus_release_id, created_seq DESC);
CREATE INDEX interpretation_revision_parent_idx
  ON interpretation_revision (parent_revision_id) WHERE parent_revision_id IS NOT NULL;
CREATE INDEX simulation_run_interpretation_idx
  ON simulation_run (interpretation_revision_id, created_seq DESC);

-- Immutable logical rows: corrections append a revision/event/release. Derived
-- projections and artifact-cache tables live outside this protected set.
CREATE FUNCTION historyworld_reject_mutation() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION '% is append-only; create a revision or auditable event', TG_TABLE_NAME;
END;
$$;

DO $$
DECLARE
  table_name text;
BEGIN
  FOREACH table_name IN ARRAY ARRAY[
    'reference_frame', 'temporal_extent', 'spatial_extent', 'source_document',
    'source_revision', 'entity', 'predicate_schema', 'hypothesis_set',
    'hypothesis_member', 'claim', 'claim_revision',
    'claim_revision_parent', 'claim_spatial_extent', 'citation', 'claim_evidence',
    'change_set', 'change_set_item', 'review_event', 'moderation_event',
    'vote_signal', 'corpus_release', 'corpus_release_claim',
    'interpretation_revision', 'derivation_activity', 'derivation_input',
    'derivation_output', 'simulation_run'
  ]
  LOOP
    EXECUTE format(
      'CREATE TRIGGER %I_append_only BEFORE UPDATE OR DELETE ON %I '
      'FOR EACH ROW EXECUTE FUNCTION historyworld_reject_mutation()',
      table_name,
      table_name
    );
  END LOOP;
END;
$$;

COMMIT;
