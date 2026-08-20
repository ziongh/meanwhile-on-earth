//! Deterministic, demand-driven world-state materialization.
//!
//! There is intentionally no global frame loop. A request evaluates only one
//! cell's ancestry, under finite work/byte/instance budgets. Renderers turn the
//! resulting semantic state into actors, PCG graphs, Mass entities, or future
//! engine facilities without feeding camera state back into canonical history.

use historyworld_content::{
    CANONICAL_ENCODING, CONTENT_COMPILER_VERSION, CONTENT_SCHEMA_VERSION, CompiledRelease,
    ContentError, SemanticPatch,
};
use historyworld_domain::{
    ClaimRef, ContentHash, DetailLevel, EpistemicClass, EvidenceRef, LayerMask, ResourceBudget,
    StableId, SurfaceState, WorldCell, WorldInstant,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MATERIALIZER_VERSION: &str = "historyworld-materializer-v1.0.0-pre.1";
pub const NUMERIC_MODEL_VERSION: &str = "fixed-point-si-v1";
pub const PRNG_VERSION: &str = "splitmix64-v1";

const SECONDS_PER_DAY: i64 = 86_400;
const SECONDS_PER_MEAN_YEAR: i64 = 31_556_952;
const BYTES_PER_APPLIED_PATCH_UPPER_BOUND: u64 = 256;
const BYTES_PER_PROVENANCE_REF_UPPER_BOUND: u64 = 512;
const SNAPSHOT_BASE_BYTES_UPPER_BOUND: u64 = 4_096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnapshotRequest {
    pub cell: WorldCell,
    pub at: WorldInstant,
    pub layers: LayerMask,
    pub requested_detail: DetailLevel,
    pub budget: ResourceBudget,
}

/// A provenance edge retained in the semantic response.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppliedPatch {
    pub patch_id: StableId,
    pub epistemic_class: EpistemicClass,
    pub claims: Vec<ClaimRef>,
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DegradationReason {
    CellResolution,
    InstanceBudget,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SnapshotMetrics {
    pub candidate_patches: u32,
    pub applied_patches: u32,
    pub work_units: u64,
    pub conservative_output_bound_bytes: u64,
    pub suggested_instance_ceiling: u32,
}

/// Maximal interval around the query with no selected-layer patch boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StableTimeSegment {
    pub start_inclusive: Option<WorldInstant>,
    pub end_exclusive: Option<WorldInstant>,
}

/// Immutable semantic state. It contains no meshes, Actors, or Unreal types.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub contract: String,
    pub materializer_version: String,
    pub numeric_model_version: String,
    pub prng_version: String,
    pub release_id: ContentHash,
    pub local_dependency_hash: ContentHash,
    pub semantic_state_hash: ContentHash,
    pub cell: WorldCell,
    pub evaluated_at: WorldInstant,
    pub stable_time_segment: StableTimeSegment,
    pub realization_epoch: WorldInstant,
    pub requested_detail: DetailLevel,
    pub effective_detail: DetailLevel,
    pub degradation_reasons: Vec<DegradationReason>,
    pub layers: LayerMask,
    pub surface: SurfaceState,
    pub provenance: Vec<AppliedPatch>,
    pub realization_seed: ContentHash,
    pub metrics: SnapshotMetrics,
}

#[derive(Clone, Debug)]
pub struct Materializer {
    release: CompiledRelease,
}

impl Materializer {
    pub const fn new(release: CompiledRelease) -> Self {
        Self { release }
    }

    pub const fn release(&self) -> &CompiledRelease {
        &self.release
    }

    pub fn snapshot(&self, request: SnapshotRequest) -> Result<WorldSnapshot, SimulationError> {
        let admission = admit_snapshot(&self.release, request)?;
        let preflight = admission.preflight;

        let (effective_detail, degradation_reasons) = choose_detail(
            request.cell,
            request.requested_detail,
            request.budget.max_generated_instances,
        );
        let suggested_instance_ceiling =
            instance_ceiling(effective_detail).min(request.budget.max_generated_instances);
        let realization_epoch = request.at.bucket_start(time_quantum(effective_detail))?;

        let mut surface = SurfaceState::default();
        let mut provenance = Vec::with_capacity(preflight.active_patches as usize);
        for cell in request.cell.ancestors_root_first() {
            for layer in request.layers.iter() {
                for patch in self.release.patches_for_layer(cell, layer) {
                    if patch.validity.contains(request.at) {
                        patch.operation.apply(&mut surface)?;
                        provenance.push(applied_patch(patch));
                    }
                }
            }
        }
        surface.validate()?;

        let applied_count = u32::try_from(provenance.len()).unwrap_or(u32::MAX);
        debug_assert_eq!(applied_count, preflight.active_patches);

        let release_id = self.release.manifest().release_id;
        let local_dependency_hash = preflight.local_dependency_hash;
        let realization_seed = derive_seed(
            local_dependency_hash,
            request.cell,
            realization_epoch,
            effective_detail,
            request.layers,
            "realization-root",
            0,
        );
        let semantic_state_hash = hash_semantic_state(
            local_dependency_hash,
            request.cell,
            request.layers,
            &surface,
            &provenance,
        );

        Ok(WorldSnapshot {
            contract: "historyworld.snapshot.v1".to_owned(),
            materializer_version: MATERIALIZER_VERSION.to_owned(),
            numeric_model_version: NUMERIC_MODEL_VERSION.to_owned(),
            prng_version: PRNG_VERSION.to_owned(),
            release_id,
            local_dependency_hash,
            semantic_state_hash,
            cell: request.cell,
            evaluated_at: request.at,
            stable_time_segment: preflight.stable_time_segment,
            realization_epoch,
            requested_detail: request.requested_detail,
            effective_detail,
            degradation_reasons,
            layers: request.layers,
            surface,
            provenance,
            realization_seed,
            metrics: SnapshotMetrics {
                candidate_patches: admission.candidate_patches,
                applied_patches: applied_count,
                work_units: admission.work_units,
                conservative_output_bound_bytes: admission.output_bound,
                suggested_instance_ceiling,
            },
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SnapshotAdmission {
    candidate_patches: u32,
    work_units: u64,
    output_bound: u64,
    preflight: Preflight,
}

fn admit_snapshot(
    release: &CompiledRelease,
    request: SnapshotRequest,
) -> Result<SnapshotAdmission, SimulationError> {
    request.budget.validate()?;
    let candidate_count = release.candidate_count_for(request.cell, request.layers);
    let candidate_patches = u32::try_from(candidate_count).unwrap_or(u32::MAX);
    if candidate_patches > request.budget.max_candidate_patches {
        return Err(SimulationError::PatchBudgetExceeded {
            actual: candidate_patches,
            limit: request.budget.max_candidate_patches,
        });
    }
    let minimum_work = estimate_work(request.cell, candidate_count, 0);
    if minimum_work > request.budget.max_work_units {
        return Err(SimulationError::WorkBudgetExceeded {
            actual: minimum_work,
            limit: request.budget.max_work_units,
        });
    }
    let base_output_bound = conservative_output_bound(0, 0);
    if base_output_bound > u64::from(request.budget.max_output_bytes) {
        return Err(SimulationError::OutputBudgetExceeded {
            conservative_bound: base_output_bound,
            limit: request.budget.max_output_bytes,
        });
    }
    let preflight = preflight(release, request, candidate_count)?;
    Ok(SnapshotAdmission {
        candidate_patches,
        work_units: estimate_work(
            request.cell,
            candidate_count,
            preflight.provenance_references,
        ),
        output_bound: conservative_output_bound(
            preflight.active_patches,
            preflight.provenance_references,
        ),
        preflight,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Preflight {
    active_patches: u32,
    provenance_references: u64,
    stable_time_segment: StableTimeSegment,
    local_dependency_hash: ContentHash,
}

fn preflight(
    release: &CompiledRelease,
    request: SnapshotRequest,
    candidate_count: usize,
) -> Result<Preflight, SimulationError> {
    let mut dependency_hasher = blake3::Hasher::new();
    dependency_hasher.update(b"historyworld.local-dependencies\0");
    dependency_hasher.update(CANONICAL_ENCODING.as_bytes());
    dependency_hasher.update(CONTENT_COMPILER_VERSION.as_bytes());
    dependency_hasher.update(&CONTENT_SCHEMA_VERSION.to_le_bytes());
    put_string(
        &mut dependency_hasher,
        &release.manifest().world_clock_version,
    );
    put_string(
        &mut dependency_hasher,
        &release.manifest().cell_scheme_version,
    );
    dependency_hasher.update(&request.cell.raw().to_le_bytes());
    dependency_hasher.update(&request.layers.bits().to_le_bytes());
    let mut result = Preflight {
        active_patches: 0,
        provenance_references: 0,
        stable_time_segment: StableTimeSegment {
            start_inclusive: None,
            end_exclusive: None,
        },
        local_dependency_hash: ContentHash::ZERO,
    };
    for cell in request.cell.ancestors_root_first() {
        for layer in request.layers.iter() {
            for patch in release.patches_for_layer(cell, layer) {
                observe_boundary(
                    &mut result.stable_time_segment,
                    request.at,
                    patch.validity.start,
                );
                if let Some(end) = patch.validity.end {
                    observe_boundary(&mut result.stable_time_segment, request.at, end);
                }
                if patch.validity.contains(request.at) {
                    result.active_patches = result.active_patches.saturating_add(1);
                    let references = patch.claims.len().saturating_add(patch.evidence.len());
                    result.provenance_references = result
                        .provenance_references
                        .saturating_add(u64::try_from(references).unwrap_or(u64::MAX));
                    let work =
                        estimate_work(request.cell, candidate_count, result.provenance_references);
                    if work > request.budget.max_work_units {
                        return Err(SimulationError::WorkBudgetExceeded {
                            actual: work,
                            limit: request.budget.max_work_units,
                        });
                    }
                    let output_bound = conservative_output_bound(
                        result.active_patches,
                        result.provenance_references,
                    );
                    if output_bound > u64::from(request.budget.max_output_bytes) {
                        return Err(SimulationError::OutputBudgetExceeded {
                            conservative_bound: output_bound,
                            limit: request.budget.max_output_bytes,
                        });
                    }
                    dependency_hasher.update(&cell.raw().to_le_bytes());
                    dependency_hasher.update(release.patch_content_hash(patch.id)?.as_bytes());
                }
            }
        }
    }
    result.local_dependency_hash =
        ContentHash::from_bytes(*dependency_hasher.finalize().as_bytes());
    Ok(result)
}

fn conservative_output_bound(active_patches: u32, provenance_references: u64) -> u64 {
    SNAPSHOT_BASE_BYTES_UPPER_BOUND
        .saturating_add(
            u64::from(active_patches).saturating_mul(BYTES_PER_APPLIED_PATCH_UPPER_BOUND),
        )
        .saturating_add(provenance_references.saturating_mul(BYTES_PER_PROVENANCE_REF_UPPER_BOUND))
}

fn estimate_work(cell: WorldCell, candidate_count: usize, provenance_references: u64) -> u64 {
    let ancestor_count = u64::from(cell.level()) + 1;
    ancestor_count
        .saturating_mul(4)
        .saturating_add(
            u64::try_from(candidate_count)
                .unwrap_or(u64::MAX)
                .saturating_mul(8),
        )
        .saturating_add(provenance_references.saturating_mul(4))
}

fn observe_boundary(segment: &mut StableTimeSegment, at: WorldInstant, boundary: WorldInstant) {
    if boundary <= at {
        if segment
            .start_inclusive
            .is_none_or(|current| boundary > current)
        {
            segment.start_inclusive = Some(boundary);
        }
    } else if segment
        .end_exclusive
        .is_none_or(|current| boundary < current)
    {
        segment.end_exclusive = Some(boundary);
    }
}

fn applied_patch(patch: &SemanticPatch) -> AppliedPatch {
    AppliedPatch {
        patch_id: patch.id,
        epistemic_class: patch.epistemic_class,
        claims: patch.claims.clone(),
        evidence: patch.evidence.clone(),
    }
}

fn choose_detail(
    cell: WorldCell,
    requested: DetailLevel,
    max_instances: u32,
) -> (DetailLevel, Vec<DegradationReason>) {
    let mut effective = requested;
    let mut reasons = Vec::new();
    let cell_cap = detail_cap_for_cell(cell.level());
    if effective > cell_cap {
        effective = cell_cap;
        reasons.push(DegradationReason::CellResolution);
    }
    if instance_ceiling(effective) > max_instances {
        while instance_ceiling(effective) > max_instances {
            let Some(lower) = effective.lower() else {
                break;
            };
            effective = lower;
        }
        reasons.push(DegradationReason::InstanceBudget);
    }
    (effective, reasons)
}

const fn detail_cap_for_cell(level: u8) -> DetailLevel {
    match level {
        0..=2 => DetailLevel::Planet,
        3..=6 => DetailLevel::Continental,
        7..=12 => DetailLevel::Regional,
        13..=20 => DetailLevel::Local,
        _ => DetailLevel::Street,
    }
}

const fn instance_ceiling(detail: DetailLevel) -> u32 {
    match detail {
        DetailLevel::Planet => 1,
        DetailLevel::Continental => 16,
        DetailLevel::Regional => 256,
        DetailLevel::Local => 2_000,
        DetailLevel::Street => 20_000,
    }
}

const fn time_quantum(detail: DetailLevel) -> i64 {
    match detail {
        DetailLevel::Planet => SECONDS_PER_MEAN_YEAR * 100,
        DetailLevel::Continental => SECONDS_PER_MEAN_YEAR * 10,
        DetailLevel::Regional => SECONDS_PER_MEAN_YEAR,
        DetailLevel::Local => SECONDS_PER_DAY,
        DetailLevel::Street => 60,
    }
}

fn hash_semantic_state(
    local_dependency_hash: ContentHash,
    cell: WorldCell,
    layers: LayerMask,
    surface: &SurfaceState,
    provenance: &[AppliedPatch],
) -> ContentHash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"historyworld.semantic-state\0");
    hasher.update(MATERIALIZER_VERSION.as_bytes());
    hasher.update(NUMERIC_MODEL_VERSION.as_bytes());
    hasher.update(local_dependency_hash.as_bytes());
    hasher.update(&cell.raw().to_le_bytes());
    hasher.update(&layers.bits().to_le_bytes());
    put_optional_i64(&mut hasher, surface.mean_elevation_mm);
    put_optional_u64(&mut hasher, surface.relief_mm);
    put_optional_u32(&mut hasher, surface.land_fraction_ppm);
    put_optional_u32(&mut hasher, surface.water_fraction_ppm);
    put_optional_u32(&mut hasher, surface.vegetation_fraction_ppm);
    put_optional_i32(&mut hasher, surface.mean_temperature_millicelsius);
    put_optional_u32(&mut hasher, surface.annual_precipitation_mm);
    hasher.update(
        &u64::try_from(provenance.len())
            .unwrap_or(u64::MAX)
            .to_le_bytes(),
    );
    ContentHash::from_bytes(*hasher.finalize().as_bytes())
}

/// Derives an independent random stream. No subsystem shares mutable RNG state.
pub fn derive_seed(
    local_dependency_hash: ContentHash,
    cell: WorldCell,
    epoch: WorldInstant,
    detail: DetailLevel,
    layers: LayerMask,
    namespace: &str,
    ordinal: u64,
) -> ContentHash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"historyworld.random-stream\0");
    hasher.update(MATERIALIZER_VERSION.as_bytes());
    hasher.update(NUMERIC_MODEL_VERSION.as_bytes());
    hasher.update(PRNG_VERSION.as_bytes());
    hasher.update(local_dependency_hash.as_bytes());
    hasher.update(&cell.raw().to_le_bytes());
    hasher.update(&epoch.seconds().to_le_bytes());
    hasher.update(&epoch.nanos().to_le_bytes());
    hasher.update(&[detail as u8]);
    hasher.update(&layers.bits().to_le_bytes());
    hasher.update(&(namespace.len() as u64).to_le_bytes());
    hasher.update(namespace.as_bytes());
    hasher.update(&ordinal.to_le_bytes());
    ContentHash::from_bytes(*hasher.finalize().as_bytes())
}

fn put_string(hasher: &mut blake3::Hasher, value: &str) {
    hasher.update(&u64::try_from(value.len()).unwrap_or(u64::MAX).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn put_optional_i64(hasher: &mut blake3::Hasher, value: Option<i64>) {
    match value {
        Some(value) => {
            hasher.update(&[1]);
            hasher.update(&value.to_le_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
}

fn put_optional_i32(hasher: &mut blake3::Hasher, value: Option<i32>) {
    match value {
        Some(value) => {
            hasher.update(&[1]);
            hasher.update(&value.to_le_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
}

fn put_optional_u64(hasher: &mut blake3::Hasher, value: Option<u64>) {
    match value {
        Some(value) => {
            hasher.update(&[1]);
            hasher.update(&value.to_le_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
}

fn put_optional_u32(hasher: &mut blake3::Hasher, value: Option<u32>) {
    match value {
        Some(value) => {
            hasher.update(&[1]);
            hasher.update(&value.to_le_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
}

/// Small, version-pinned generator for cosmetic realization decisions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    pub fn from_seed(seed: ContentHash) -> Self {
        let mut bytes = [0_u8; 8];
        bytes.copy_from_slice(&seed.as_bytes()[..8]);
        Self {
            state: u64::from_le_bytes(bytes),
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }
}

#[derive(Debug, Error)]
pub enum SimulationError {
    #[error(transparent)]
    Content(#[from] ContentError),
    #[error(transparent)]
    Domain(#[from] historyworld_domain::DomainError),
    #[error("candidate patch budget exceeded: {actual} > {limit}")]
    PatchBudgetExceeded { actual: u32, limit: u32 },
    #[error("work budget exceeded: {actual} > {limit}")]
    WorkBudgetExceeded { actual: u64, limit: u64 },
    #[error("output byte budget exceeded: conservative bound {conservative_bound} > {limit}")]
    OutputBudgetExceeded { conservative_bound: u64, limit: u32 },
}

#[cfg(test)]
mod tests {
    use historyworld_content::{CONTENT_SCHEMA_VERSION, PatchOperation, ReleaseDocument};
    use historyworld_domain::{ContentHash, TimeRange};

    use super::*;

    fn patch(id: u128, cell: WorldCell, start: i64, order: u32, value: i64) -> SemanticPatch {
        SemanticPatch {
            id: StableId::from_u128(id),
            cell,
            validity: TimeRange::from_start(WorldInstant::from_seconds(start)),
            canonical_order: order,
            epistemic_class: EpistemicClass::ArtisticAmbient,
            confidence: None,
            claims: vec![],
            evidence: vec![],
            operation: PatchOperation::SetMeanElevationMm(value),
        }
    }

    fn release(patches: Vec<SemanticPatch>) -> CompiledRelease {
        CompiledRelease::compile(ReleaseDocument {
            schema_version: CONTENT_SCHEMA_VERSION,
            label: "simulation test".to_owned(),
            world_clock_version: historyworld_content::SUPPORTED_WORLD_CLOCK.to_owned(),
            cell_scheme_version: historyworld_content::SUPPORTED_CELL_SCHEME.to_owned(),
            corpus_release: ContentHash::from_bytes([1; 32]),
            interpretation_revision: ContentHash::from_bytes([2; 32]),
            parent_release: None,
            patches,
        })
        .unwrap()
    }

    fn request(cell: WorldCell, at: i64) -> SnapshotRequest {
        SnapshotRequest {
            cell,
            at: WorldInstant::from_seconds(at),
            layers: LayerMask::only(historyworld_domain::LayerKind::Geology),
            requested_detail: DetailLevel::Street,
            budget: ResourceBudget::INTERACTIVE,
        }
    }

    #[test]
    fn time_and_specificity_select_the_same_state_repeatedly() {
        let root = WorldCell::new(0, 0, 0).unwrap();
        let target = root.child(1).unwrap().child(2).unwrap();
        let materializer = Materializer::new(release(vec![
            patch(1, root, 0, 1, 100),
            patch(2, target, 50, 2, 200),
        ]));

        let before = materializer.snapshot(request(target, 49)).unwrap();
        let after = materializer.snapshot(request(target, 50)).unwrap();
        let repeated = materializer.snapshot(request(target, 50)).unwrap();
        assert_eq!(before.surface.mean_elevation_mm, Some(100));
        assert_eq!(after.surface.mean_elevation_mm, Some(200));
        assert_eq!(after, repeated);
    }

    #[test]
    fn finite_patch_budget_fails_before_unbounded_work() {
        let root = WorldCell::new(0, 0, 0).unwrap();
        let materializer = Materializer::new(release(vec![
            patch(1, root, 0, 1, 100),
            patch(2, root, 0, 2, 200),
        ]));
        let mut bounded = request(root, 1);
        bounded.budget.max_candidate_patches = 1;
        assert!(matches!(
            materializer.snapshot(bounded),
            Err(SimulationError::PatchBudgetExceeded { .. })
        ));
    }

    #[test]
    fn renderer_detail_downgrades_without_changing_semantic_state() {
        let cell = WorldCell::new(0, 18, 0).unwrap();
        let materializer = Materializer::new(release(vec![patch(1, cell, 0, 1, 100)]));
        let high = materializer.snapshot(request(cell, 1)).unwrap();
        let mut low_request = request(cell, 1);
        low_request.budget.max_generated_instances = 16;
        let low = materializer.snapshot(low_request).unwrap();
        assert_eq!(high.surface, low.surface);
        assert_eq!(high.semantic_state_hash, low.semantic_state_hash);
        assert_eq!(low.effective_detail, DetailLevel::Continental);
    }

    #[test]
    fn random_streams_are_repeatable_and_namespaced() {
        let release = ContentHash::from_bytes([7; 32]);
        let cell = WorldCell::new(2, 4, 42).unwrap();
        let seed_a = derive_seed(
            release,
            cell,
            WorldInstant::from_seconds(-1),
            DetailLevel::Regional,
            LayerMask::ALL,
            "trees",
            0,
        );
        let seed_b = derive_seed(
            release,
            cell,
            WorldInstant::from_seconds(-1),
            DetailLevel::Regional,
            LayerMask::ALL,
            "people",
            0,
        );
        assert_ne!(seed_a, seed_b);
        let mut first = DeterministicRng::from_seed(seed_a);
        let mut second = DeterministicRng::from_seed(seed_a);
        assert_eq!(first.next_u64(), second.next_u64());
        assert_eq!(first.next_u64(), second.next_u64());
    }

    #[test]
    fn unrelated_and_future_content_preserve_local_state_and_realization() {
        let target = WorldCell::new(0, 2, 1).unwrap();
        let unrelated = WorldCell::new(1, 2, 1).unwrap();
        let baseline = Materializer::new(release(vec![patch(1, target, 0, 1, 100)]));
        let expanded = Materializer::new(release(vec![
            patch(1, target, 0, 1, 100),
            patch(2, unrelated, 0, 1, 999),
            patch(3, target, 1_000_000, 2, 200),
        ]));
        let before = baseline.snapshot(request(target, 100)).unwrap();
        let after = expanded.snapshot(request(target, 100)).unwrap();

        assert_ne!(before.release_id, after.release_id);
        assert_eq!(before.local_dependency_hash, after.local_dependency_hash);
        assert_eq!(before.semantic_state_hash, after.semantic_state_hash);
        assert_eq!(before.realization_seed, after.realization_seed);
        assert_ne!(before.stable_time_segment, after.stable_time_segment);
    }
}
