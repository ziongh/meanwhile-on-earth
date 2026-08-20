//! Compilation of authored semantic patches into immutable releases.
//!
//! The contribution database is not queried by the simulator. A moderation
//! cutoff and interpretation are compiled into this deterministic, read-only
//! representation first.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use historyworld_domain::{
    ClaimRef, ConfidenceBand, ContentHash, DomainError, EpistemicClass, EvidenceRef, EvidenceRole,
    LayerKind, LayerMask, StableId, SurfaceState, TimeRange, WorldCell, WorldInstant,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CONTENT_SCHEMA_VERSION: u16 = 1;
pub const CONTENT_COMPILER_VERSION: &str = "historyworld-content-compiler-v1.0.0-pre.1";
pub const CANONICAL_ENCODING: &str = "historyworld-canonical-binary-v1";
pub const SUPPORTED_WORLD_CLOCK: &str = "hw-clock-proleptic-gregorian-v1";
pub const SUPPORTED_CELL_SCHEME: &str = "hw-cube-sphere-v1";
pub const MAX_CLAIM_REFS_PER_PATCH: usize = 64;
pub const MAX_EVIDENCE_REFS_PER_PATCH: usize = 64;
pub const MAX_PATCHES_PER_RELEASE: usize = 65_536;
pub const MAX_CANDIDATE_PATCHES_PER_CELL: usize = 4_096;
pub const MAX_COMPILER_VALIDATION_WORK_UNITS: u64 = 10_000_000;
pub const MAX_RELEASE_LABEL_BYTES: usize = 512;
pub const MAX_VERSION_IDENTIFIER_BYTES: usize = 128;
pub const MAX_ASSESSMENT_METHOD_BYTES: usize = 256;

/// A deterministic, scenario-resolved semantic change.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticPatch {
    pub id: StableId,
    pub cell: WorldCell,
    pub validity: TimeRange,
    /// Explicit order produced by the interpretation compiler. Ties use `id`.
    pub canonical_order: u32,
    pub epistemic_class: EpistemicClass,
    pub confidence: Option<ConfidenceBand>,
    #[serde(default)]
    pub claims: Vec<ClaimRef>,
    #[serde(default)]
    pub evidence: Vec<EvidenceRef>,
    pub operation: PatchOperation,
}

impl SemanticPatch {
    pub fn layer(&self) -> LayerKind {
        self.operation.layer()
    }

    fn normalize(&mut self) {
        self.claims.sort_by_key(|reference| {
            (
                reference.claim_id,
                reference.revision,
                reference.content_hash,
            )
        });
        self.evidence.sort_by_key(|reference| {
            (
                reference.citation_id,
                reference.source_revision_id,
                reference.content_hash,
                reference.role,
            )
        });
    }

    fn validate(&self) -> Result<(), ContentError> {
        self.validity.validate()?;
        if self.claims.len() > MAX_CLAIM_REFS_PER_PATCH
            || self.evidence.len() > MAX_EVIDENCE_REFS_PER_PATCH
        {
            return Err(ContentError::TooManyProvenanceReferences(self.id));
        }
        if let Some(confidence) = &self.confidence {
            confidence.validate()?;
            if confidence.assessment_method.trim().is_empty() {
                return Err(ContentError::MissingAssessmentMethod);
            }
            if confidence.assessment_method.len() > MAX_ASSESSMENT_METHOD_BYTES {
                return Err(ContentError::MetadataTooLong(
                    "confidence assessment method",
                ));
            }
        }
        self.operation.validate()?;
        if self.epistemic_class != EpistemicClass::ArtisticAmbient
            && self.claims.is_empty()
            && self.evidence.is_empty()
        {
            return Err(ContentError::MissingProvenance(self.id));
        }
        Ok(())
    }
}

/// Quantized operations understood by the first geology-to-biome materializer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum PatchOperation {
    SetMeanElevationMm(i64),
    OffsetMeanElevationMm(i64),
    SetReliefMm(u64),
    SetLandFractionPpm(u32),
    SetWaterFractionPpm(u32),
    SetVegetationFractionPpm(u32),
    SetMeanTemperatureMillicelsius(i32),
    SetAnnualPrecipitationMm(u32),
}

impl PatchOperation {
    pub const fn layer(&self) -> LayerKind {
        match self {
            Self::SetMeanElevationMm(_)
            | Self::OffsetMeanElevationMm(_)
            | Self::SetReliefMm(_)
            | Self::SetLandFractionPpm(_) => LayerKind::Geology,
            Self::SetWaterFractionPpm(_) => LayerKind::Hydrology,
            Self::SetVegetationFractionPpm(_) => LayerKind::Biome,
            Self::SetMeanTemperatureMillicelsius(_) | Self::SetAnnualPrecipitationMm(_) => {
                LayerKind::Climate
            }
        }
    }

    fn validate(&self) -> Result<(), DomainError> {
        match self {
            Self::SetLandFractionPpm(value)
            | Self::SetWaterFractionPpm(value)
            | Self::SetVegetationFractionPpm(value)
                if *value > 1_000_000 =>
            {
                Err(DomainError::InvalidPartsPerMillion)
            }
            _ => Ok(()),
        }
    }

    /// Applies the operation without allocation or floating-point arithmetic.
    pub fn apply(&self, state: &mut SurfaceState) -> Result<(), ContentError> {
        match *self {
            Self::SetMeanElevationMm(value) => state.mean_elevation_mm = Some(value),
            Self::OffsetMeanElevationMm(delta) => {
                let current = state
                    .mean_elevation_mm
                    .ok_or(ContentError::MissingBaseValue("mean_elevation_mm"))?;
                state.mean_elevation_mm = Some(
                    current
                        .checked_add(delta)
                        .ok_or(ContentError::NumericOverflow)?,
                );
            }
            Self::SetReliefMm(value) => state.relief_mm = Some(value),
            Self::SetLandFractionPpm(value) => state.land_fraction_ppm = Some(value),
            Self::SetWaterFractionPpm(value) => state.water_fraction_ppm = Some(value),
            Self::SetVegetationFractionPpm(value) => {
                state.vegetation_fraction_ppm = Some(value);
            }
            Self::SetMeanTemperatureMillicelsius(value) => {
                state.mean_temperature_millicelsius = Some(value);
            }
            Self::SetAnnualPrecipitationMm(value) => {
                state.annual_precipitation_mm = Some(value);
            }
        }
        Ok(())
    }
}

/// Human-editable input to the deterministic release compiler.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseDocument {
    pub schema_version: u16,
    pub label: String,
    pub world_clock_version: String,
    pub cell_scheme_version: String,
    pub corpus_release: ContentHash,
    pub interpretation_revision: ContentHash,
    pub parent_release: Option<ContentHash>,
    pub patches: Vec<SemanticPatch>,
}

/// Small immutable metadata returned before any state chunks are requested.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReleaseManifest {
    pub release_id: ContentHash,
    pub schema_version: u16,
    pub compiler_version: String,
    pub canonical_encoding: String,
    pub label: String,
    pub world_clock_version: String,
    pub cell_scheme_version: String,
    pub corpus_release: ContentHash,
    pub interpretation_revision: ContentHash,
    pub parent_release: Option<ContentHash>,
    pub patch_count: u64,
    pub indexed_cell_count: u64,
    pub layers: Vec<LayerKind>,
}

/// A compiled release with patches grouped into stable, read-only cell slices.
type SharedPatchSlice = Arc<[Arc<SemanticPatch>]>;
type CellLayerPatchIndex = BTreeMap<(WorldCell, LayerKind), SharedPatchSlice>;

#[derive(Clone, Debug)]
pub struct CompiledRelease {
    manifest: ReleaseManifest,
    patches_by_cell: Arc<BTreeMap<WorldCell, SharedPatchSlice>>,
    patches_by_cell_layer: Arc<CellLayerPatchIndex>,
    patch_hashes: Arc<BTreeMap<StableId, ContentHash>>,
    counts_by_cell_layer: Arc<BTreeMap<(WorldCell, LayerKind), usize>>,
}

impl CompiledRelease {
    pub fn compile(mut document: ReleaseDocument) -> Result<Self, ContentError> {
        if document.schema_version != CONTENT_SCHEMA_VERSION {
            return Err(ContentError::UnsupportedSchema(document.schema_version));
        }
        if document.label.trim().is_empty() {
            return Err(ContentError::MissingReleaseMetadata);
        }
        if document.label.len() > MAX_RELEASE_LABEL_BYTES {
            return Err(ContentError::MetadataTooLong("release label"));
        }
        validate_version_identifier("world clock", &document.world_clock_version)?;
        validate_version_identifier("cell scheme", &document.cell_scheme_version)?;
        if document.world_clock_version != SUPPORTED_WORLD_CLOCK {
            return Err(ContentError::UnsupportedWorldClock(
                document.world_clock_version,
            ));
        }
        if document.cell_scheme_version != SUPPORTED_CELL_SCHEME {
            return Err(ContentError::UnsupportedCellScheme(
                document.cell_scheme_version,
            ));
        }
        if document.patches.len() > MAX_PATCHES_PER_RELEASE {
            return Err(ContentError::TooManyPatches {
                actual: document.patches.len(),
                limit: MAX_PATCHES_PER_RELEASE,
            });
        }

        let mut ids = BTreeSet::new();
        for patch in &mut document.patches {
            patch.normalize();
            patch.validate()?;
            if !ids.insert(patch.id) {
                return Err(ContentError::DuplicatePatch(patch.id));
            }
        }
        document.patches.sort_by_key(patch_sort_key);

        let release_id = hash_release(&document);
        let patch_hashes = document
            .patches
            .iter()
            .map(|patch| (patch.id, hash_patch(patch)))
            .collect::<BTreeMap<_, _>>();
        let mut grouped = BTreeMap::<WorldCell, Vec<Arc<SemanticPatch>>>::new();
        let mut grouped_by_layer =
            BTreeMap::<(WorldCell, LayerKind), Vec<Arc<SemanticPatch>>>::new();
        let mut counts_by_cell_layer = BTreeMap::<(WorldCell, LayerKind), usize>::new();
        let mut layers = BTreeSet::new();
        for patch in document.patches {
            layers.insert(patch.layer());
            *counts_by_cell_layer
                .entry((patch.cell, patch.layer()))
                .or_default() += 1;
            let cell = patch.cell;
            let layer = patch.layer();
            let patch = Arc::new(patch);
            grouped.entry(cell).or_default().push(Arc::clone(&patch));
            grouped_by_layer
                .entry((cell, layer))
                .or_default()
                .push(patch);
        }
        let patches_by_cell: BTreeMap<WorldCell, SharedPatchSlice> = grouped
            .into_iter()
            .map(|(cell, patches)| (cell, Arc::from(patches)))
            .collect();
        let patches_by_cell_layer = grouped_by_layer
            .into_iter()
            .map(|(key, patches)| (key, Arc::from(patches)))
            .collect::<BTreeMap<_, _>>();
        validate_runtime_totality(&patches_by_cell, &patches_by_cell_layer)?;
        let patch_count = patches_by_cell
            .values()
            .map(|patches| patches.len() as u64)
            .sum();
        let indexed_cell_count = patches_by_cell.len() as u64;

        Ok(Self {
            manifest: ReleaseManifest {
                release_id,
                schema_version: document.schema_version,
                compiler_version: CONTENT_COMPILER_VERSION.to_owned(),
                canonical_encoding: CANONICAL_ENCODING.to_owned(),
                label: document.label,
                world_clock_version: document.world_clock_version,
                cell_scheme_version: document.cell_scheme_version,
                corpus_release: document.corpus_release,
                interpretation_revision: document.interpretation_revision,
                parent_release: document.parent_release,
                patch_count,
                indexed_cell_count,
                layers: layers.into_iter().collect(),
            },
            patches_by_cell: Arc::new(patches_by_cell),
            patches_by_cell_layer: Arc::new(patches_by_cell_layer),
            patch_hashes: Arc::new(patch_hashes),
            counts_by_cell_layer: Arc::new(counts_by_cell_layer),
        })
    }

    pub fn from_json(json: &str) -> Result<Self, ContentError> {
        Self::compile(serde_json::from_str(json)?)
    }

    pub const fn manifest(&self) -> &ReleaseManifest {
        &self.manifest
    }

    pub fn patches_for_layer(
        &self,
        cell: WorldCell,
        layer: LayerKind,
    ) -> impl Iterator<Item = &SemanticPatch> {
        self.patches_by_cell_layer
            .get(&(cell, layer))
            .into_iter()
            .flat_map(|patches| patches.iter().map(AsRef::as_ref))
    }

    pub fn candidate_count_for(&self, cell: WorldCell, layers: LayerMask) -> usize {
        cell.ancestors_root_first()
            .flat_map(|ancestor| {
                layers.iter().map(move |layer| {
                    self.counts_by_cell_layer
                        .get(&(ancestor, layer))
                        .copied()
                        .unwrap_or(0)
                })
            })
            .sum()
    }

    pub fn all_patches(&self) -> impl Iterator<Item = &SemanticPatch> {
        self.patches_by_cell
            .values()
            .flat_map(|patches| patches.iter().map(AsRef::as_ref))
    }

    pub fn patch_content_hash(&self, id: StableId) -> Result<ContentHash, ContentError> {
        self.patch_hashes
            .get(&id)
            .copied()
            .ok_or(ContentError::MissingCompiledPatch(id))
    }
}

fn validate_version_identifier(name: &'static str, value: &str) -> Result<(), ContentError> {
    if value.trim().is_empty() {
        return Err(ContentError::MissingReleaseMetadata);
    }
    if value.len() > MAX_VERSION_IDENTIFIER_BYTES {
        return Err(ContentError::MetadataTooLong(name));
    }
    Ok(())
}

/// Proves that every state transition in each indexed ancestry can be queried
/// without a missing base value, fixed-point overflow, or invalid surface.
fn validate_runtime_totality(
    patches_by_cell: &BTreeMap<WorldCell, SharedPatchSlice>,
    patches_by_cell_layer: &CellLayerPatchIndex,
) -> Result<(), ContentError> {
    let mut validation_work_units = 0_u64;
    let offset_cells = patches_by_cell
        .iter()
        .filter_map(|(cell, patches)| {
            patches
                .iter()
                .any(|patch| matches!(patch.operation, PatchOperation::OffsetMeanElevationMm(_)))
                .then_some(*cell)
        })
        .collect::<BTreeSet<_>>();
    for target in patches_by_cell.keys().copied() {
        let candidate_count = target
            .ancestors_root_first()
            .flat_map(|cell| {
                LayerKind::ALL.into_iter().map(move |layer| {
                    patches_by_cell_layer
                        .get(&(cell, layer))
                        .map_or(0, |patches| patches.len())
                })
            })
            .sum::<usize>();
        if candidate_count > MAX_CANDIDATE_PATCHES_PER_CELL {
            return Err(ContentError::TooManyCellCandidates {
                cell: target,
                actual: candidate_count,
                limit: MAX_CANDIDATE_PATCHES_PER_CELL,
            });
        }
        if !target
            .ancestors_root_first()
            .any(|ancestor| offset_cells.contains(&ancestor))
        {
            continue;
        }
        let candidates = target
            .ancestors_root_first()
            .flat_map(|cell| {
                patches_by_cell_layer
                    .get(&(cell, LayerKind::Geology))
                    .into_iter()
                    .flat_map(|patches| patches.iter().map(AsRef::as_ref))
            })
            .filter(|patch| {
                matches!(
                    patch.operation,
                    PatchOperation::SetMeanElevationMm(_)
                        | PatchOperation::OffsetMeanElevationMm(_)
                )
            })
            .collect::<Vec<_>>();
        validation_work_units = validation_work_units
            .saturating_add(u64::try_from(candidates.len()).unwrap_or(u64::MAX));
        if validation_work_units > MAX_COMPILER_VALIDATION_WORK_UNITS {
            return Err(ContentError::CompilerWorkBudgetExceeded {
                actual: validation_work_units,
                limit: MAX_COMPILER_VALIDATION_WORK_UNITS,
            });
        }
        let boundaries = candidates
            .iter()
            .flat_map(|patch| [Some(patch.validity.start), patch.validity.end])
            .flatten()
            .collect::<BTreeSet<_>>();
        let target_work = u64::try_from(candidates.len())
            .unwrap_or(u64::MAX)
            .saturating_mul(u64::try_from(boundaries.len()).unwrap_or(u64::MAX));
        validation_work_units = validation_work_units.saturating_add(target_work);
        if validation_work_units > MAX_COMPILER_VALIDATION_WORK_UNITS {
            return Err(ContentError::CompilerWorkBudgetExceeded {
                actual: validation_work_units,
                limit: MAX_COMPILER_VALIDATION_WORK_UNITS,
            });
        }
        for at in boundaries {
            let mut surface = SurfaceState::default();
            for patch in &candidates {
                if patch.validity.contains(at) {
                    patch.operation.apply(&mut surface).map_err(|error| {
                        ContentError::ReleaseNotRuntimeTotal {
                            cell: target,
                            at,
                            reason: error.to_string(),
                        }
                    })?;
                }
            }
            surface
                .validate()
                .map_err(|error| ContentError::ReleaseNotRuntimeTotal {
                    cell: target,
                    at,
                    reason: error.to_string(),
                })?;
        }
    }
    Ok(())
}

fn patch_sort_key(patch: &SemanticPatch) -> (u64, u32, StableId) {
    (patch.cell.raw(), patch.canonical_order, patch.id)
}

fn hash_release(document: &ReleaseDocument) -> ContentHash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"historyworld.semantic-release\0");
    hasher.update(CANONICAL_ENCODING.as_bytes());
    hasher.update(CONTENT_COMPILER_VERSION.as_bytes());
    put_u16(&mut hasher, document.schema_version);
    put_string(&mut hasher, &document.label);
    put_string(&mut hasher, &document.world_clock_version);
    put_string(&mut hasher, &document.cell_scheme_version);
    hasher.update(document.corpus_release.as_bytes());
    hasher.update(document.interpretation_revision.as_bytes());
    put_optional_hash(&mut hasher, document.parent_release);
    put_u64(&mut hasher, document.patches.len() as u64);
    for patch in &document.patches {
        put_patch(&mut hasher, patch);
    }
    ContentHash::from_bytes(*hasher.finalize().as_bytes())
}

fn hash_patch(patch: &SemanticPatch) -> ContentHash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"historyworld.semantic-patch\0");
    hasher.update(CANONICAL_ENCODING.as_bytes());
    put_patch(&mut hasher, patch);
    ContentHash::from_bytes(*hasher.finalize().as_bytes())
}

fn put_patch(hasher: &mut blake3::Hasher, patch: &SemanticPatch) {
    hasher.update(&patch.id.to_be_bytes());
    put_u64(hasher, patch.cell.raw());
    put_instant(hasher, patch.validity.start);
    put_optional_instant(hasher, patch.validity.end);
    put_u32(hasher, patch.canonical_order);
    put_u8(hasher, epistemic_tag(patch.epistemic_class));
    match &patch.confidence {
        Some(confidence) => {
            put_u8(hasher, 1);
            put_u32(hasher, confidence.lower_ppm);
            put_u32(hasher, confidence.preferred_ppm);
            put_u32(hasher, confidence.upper_ppm);
            put_string(hasher, &confidence.assessment_method);
        }
        None => put_u8(hasher, 0),
    }
    put_u64(hasher, patch.claims.len() as u64);
    for claim in &patch.claims {
        hasher.update(&claim.claim_id.to_be_bytes());
        put_u32(hasher, claim.revision);
        hasher.update(claim.content_hash.as_bytes());
    }
    put_u64(hasher, patch.evidence.len() as u64);
    for evidence in &patch.evidence {
        hasher.update(&evidence.citation_id.to_be_bytes());
        hasher.update(&evidence.source_revision_id.to_be_bytes());
        hasher.update(evidence.content_hash.as_bytes());
        put_u8(hasher, evidence_role_tag(evidence.role));
    }
    put_operation(hasher, &patch.operation);
}

fn put_operation(hasher: &mut blake3::Hasher, operation: &PatchOperation) {
    match *operation {
        PatchOperation::SetMeanElevationMm(value) => {
            put_u8(hasher, 0);
            hasher.update(&value.to_le_bytes());
        }
        PatchOperation::OffsetMeanElevationMm(value) => {
            put_u8(hasher, 1);
            hasher.update(&value.to_le_bytes());
        }
        PatchOperation::SetReliefMm(value) => {
            put_u8(hasher, 2);
            put_u64(hasher, value);
        }
        PatchOperation::SetLandFractionPpm(value) => {
            put_u8(hasher, 3);
            put_u32(hasher, value);
        }
        PatchOperation::SetWaterFractionPpm(value) => {
            put_u8(hasher, 4);
            put_u32(hasher, value);
        }
        PatchOperation::SetVegetationFractionPpm(value) => {
            put_u8(hasher, 5);
            put_u32(hasher, value);
        }
        PatchOperation::SetMeanTemperatureMillicelsius(value) => {
            put_u8(hasher, 6);
            hasher.update(&value.to_le_bytes());
        }
        PatchOperation::SetAnnualPrecipitationMm(value) => {
            put_u8(hasher, 7);
            put_u32(hasher, value);
        }
    }
}

const fn epistemic_tag(value: EpistemicClass) -> u8 {
    match value {
        EpistemicClass::DirectObservation => 0,
        EpistemicClass::ReportedPrimarySource => 1,
        EpistemicClass::ScholarlyInference => 2,
        EpistemicClass::ModelDerived => 3,
        EpistemicClass::PlausibleReconstruction => 4,
        EpistemicClass::ArtisticAmbient => 5,
    }
}

const fn evidence_role_tag(value: EvidenceRole) -> u8 {
    match value {
        EvidenceRole::Supports => 0,
        EvidenceRole::Contradicts => 1,
        EvidenceRole::Contextualizes => 2,
    }
}

fn put_instant(hasher: &mut blake3::Hasher, instant: WorldInstant) {
    hasher.update(&instant.seconds().to_le_bytes());
    put_u32(hasher, instant.nanos());
}

fn put_optional_instant(hasher: &mut blake3::Hasher, value: Option<WorldInstant>) {
    match value {
        Some(value) => {
            put_u8(hasher, 1);
            put_instant(hasher, value);
        }
        None => put_u8(hasher, 0),
    }
}

fn put_optional_hash(hasher: &mut blake3::Hasher, value: Option<ContentHash>) {
    match value {
        Some(value) => {
            put_u8(hasher, 1);
            hasher.update(value.as_bytes());
        }
        None => put_u8(hasher, 0),
    }
}

fn put_string(hasher: &mut blake3::Hasher, value: &str) {
    put_u64(hasher, value.len() as u64);
    hasher.update(value.as_bytes());
}

fn put_u8(hasher: &mut blake3::Hasher, value: u8) {
    hasher.update(&[value]);
}

fn put_u16(hasher: &mut blake3::Hasher, value: u16) {
    hasher.update(&value.to_le_bytes());
}

fn put_u32(hasher: &mut blake3::Hasher, value: u32) {
    hasher.update(&value.to_le_bytes());
}

fn put_u64(hasher: &mut blake3::Hasher, value: u64) {
    hasher.update(&value.to_le_bytes());
}

#[derive(Debug, Error)]
pub enum ContentError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("unsupported content schema version {0}")]
    UnsupportedSchema(u16),
    #[error("release label and version identifiers must not be empty")]
    MissingReleaseMetadata,
    #[error("{0} exceeds its canonical byte limit")]
    MetadataTooLong(&'static str),
    #[error("confidence assessment method must not be empty")]
    MissingAssessmentMethod,
    #[error("unsupported world clock version '{0}'")]
    UnsupportedWorldClock(String),
    #[error("unsupported cell scheme version '{0}'")]
    UnsupportedCellScheme(String),
    #[error("release has {actual} patches, above the compiler limit of {limit}")]
    TooManyPatches { actual: usize, limit: usize },
    #[error("cell {cell} has {actual} candidate patches, above the compiler limit of {limit}")]
    TooManyCellCandidates {
        cell: WorldCell,
        actual: usize,
        limit: usize,
    },
    #[error("release validation requires {actual} work units, above the compiler limit of {limit}")]
    CompilerWorkBudgetExceeded { actual: u64, limit: u64 },
    #[error("duplicate semantic patch {0}")]
    DuplicatePatch(StableId),
    #[error("non-artistic patch {0} has no claim or evidence provenance")]
    MissingProvenance(StableId),
    #[error("patch {0} exceeds the bounded claim/evidence reference count")]
    TooManyProvenanceReferences(StableId),
    #[error("operation requires a known base value for {0}")]
    MissingBaseValue(&'static str),
    #[error("fixed-point arithmetic overflowed")]
    NumericOverflow,
    #[error("compiled release is not query-total at {cell}, {at}: {reason}")]
    ReleaseNotRuntimeTotal {
        cell: WorldCell,
        at: WorldInstant,
        reason: String,
    },
    #[error("compiled release omitted the content hash for patch {0}")]
    MissingCompiledPatch(StableId),
}

#[cfg(test)]
mod tests {
    use historyworld_domain::{ContentHash, EpistemicClass, TimeRange, WorldInstant};

    use super::*;

    fn artistic_patch(id: u128, order: u32) -> SemanticPatch {
        SemanticPatch {
            id: StableId::from_u128(id),
            cell: WorldCell::new(0, 0, 0).unwrap(),
            validity: TimeRange::from_start(WorldInstant::from_seconds(0)),
            canonical_order: order,
            epistemic_class: EpistemicClass::ArtisticAmbient,
            confidence: None,
            claims: vec![],
            evidence: vec![],
            operation: PatchOperation::SetMeanElevationMm(i64::try_from(id).unwrap()),
        }
    }

    fn document(patches: Vec<SemanticPatch>) -> ReleaseDocument {
        ReleaseDocument {
            schema_version: CONTENT_SCHEMA_VERSION,
            label: "test".to_owned(),
            world_clock_version: SUPPORTED_WORLD_CLOCK.to_owned(),
            cell_scheme_version: SUPPORTED_CELL_SCHEME.to_owned(),
            corpus_release: ContentHash::from_bytes([1; 32]),
            interpretation_revision: ContentHash::from_bytes([2; 32]),
            parent_release: None,
            patches,
        }
    }

    #[test]
    fn input_order_does_not_change_release_hash() {
        let first =
            CompiledRelease::compile(document(vec![artistic_patch(1, 1), artistic_patch(2, 2)]))
                .unwrap();
        let second =
            CompiledRelease::compile(document(vec![artistic_patch(2, 2), artistic_patch(1, 1)]))
                .unwrap();
        assert_eq!(first.manifest().release_id, second.manifest().release_id);
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let error =
            CompiledRelease::compile(document(vec![artistic_patch(1, 1), artistic_patch(1, 2)]))
                .unwrap_err();
        assert!(matches!(error, ContentError::DuplicatePatch(_)));
    }

    #[test]
    fn equal_logical_provenance_keys_have_a_total_canonical_order() {
        let mut first_patch = artistic_patch(1, 1);
        first_patch.claims = vec![
            ClaimRef {
                claim_id: StableId::from_u128(7),
                revision: 1,
                content_hash: ContentHash::from_bytes([1; 32]),
            },
            ClaimRef {
                claim_id: StableId::from_u128(7),
                revision: 1,
                content_hash: ContentHash::from_bytes([2; 32]),
            },
        ];
        let mut reversed_patch = first_patch.clone();
        reversed_patch.claims.reverse();
        let first = CompiledRelease::compile(document(vec![first_patch])).unwrap();
        let reversed = CompiledRelease::compile(document(vec![reversed_patch])).unwrap();
        assert_eq!(first.manifest().release_id, reversed.manifest().release_id);
    }

    #[test]
    fn release_validation_proves_offset_operations_are_query_total() {
        let root = WorldCell::new(0, 0, 0).unwrap();
        let mut offset = artistic_patch(1, 1);
        offset.cell = root;
        offset.operation = PatchOperation::OffsetMeanElevationMm(1);
        let error = CompiledRelease::compile(document(vec![offset])).unwrap_err();
        assert!(matches!(error, ContentError::ReleaseNotRuntimeTotal { .. }));

        let mut base = artistic_patch(2, 0);
        base.cell = root;
        base.operation = PatchOperation::SetMeanElevationMm(10);
        assert!(CompiledRelease::compile(document(vec![offset_for_test(), base])).is_ok());
    }

    fn offset_for_test() -> SemanticPatch {
        let mut offset = artistic_patch(1, 1);
        offset.operation = PatchOperation::OffsetMeanElevationMm(1);
        offset
    }

    #[test]
    fn unknown_fields_and_unsupported_coordinate_contracts_fail_closed() {
        let unknown = r#"{
            "schema_version":1,
            "label":"test",
            "world_clock_version":"hw-clock-proleptic-gregorian-v1",
            "cell_scheme_version":"hw-cube-sphere-v1",
            "corpus_release":"1111111111111111111111111111111111111111111111111111111111111111",
            "interpretation_revision":"2222222222222222222222222222222222222222222222222222222222222222",
            "parent_release":null,
            "patches":[],
            "typo":true
        }"#;
        assert!(CompiledRelease::from_json(unknown).is_err());

        let mut unsupported = document(vec![]);
        unsupported.world_clock_version = "future-clock".to_owned();
        assert!(matches!(
            CompiledRelease::compile(unsupported),
            Err(ContentError::UnsupportedWorldClock(_))
        ));
    }
}
