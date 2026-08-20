use std::{hint::black_box, time::Instant};

use historyworld_content::{
    CONTENT_SCHEMA_VERSION, CompiledRelease, PatchOperation, ReleaseDocument, SemanticPatch,
};
use historyworld_domain::{
    ContentHash, DetailLevel, EpistemicClass, LayerKind, LayerMask, ResourceBudget, StableId,
    TimeRange, WorldCell, WorldInstant,
};
use historyworld_sim::{Materializer, SnapshotRequest};

fn main() {
    let iterations = std::env::var("HW_BENCH_ITERS")
        .ok()
        .and_then(|value| value.parse::<u128>().ok())
        .unwrap_or(10_000)
        .max(1);
    let cell = WorldCell::new(0, 14, 0).unwrap();
    let patches = (0_u32..256)
        .map(|index| SemanticPatch {
            id: StableId::from_u128(u128::from(index) + 1),
            cell,
            validity: TimeRange::from_start(WorldInstant::from_seconds(i64::from(index))),
            canonical_order: index,
            epistemic_class: EpistemicClass::ArtisticAmbient,
            confidence: None,
            claims: vec![],
            evidence: vec![],
            operation: PatchOperation::SetMeanElevationMm(i64::from(index)),
        })
        .collect();
    let release = CompiledRelease::compile(ReleaseDocument {
        schema_version: CONTENT_SCHEMA_VERSION,
        label: "materializer benchmark".to_owned(),
        world_clock_version: historyworld_content::SUPPORTED_WORLD_CLOCK.to_owned(),
        cell_scheme_version: historyworld_content::SUPPORTED_CELL_SCHEME.to_owned(),
        corpus_release: ContentHash::from_bytes([1; 32]),
        interpretation_revision: ContentHash::from_bytes([2; 32]),
        parent_release: None,
        patches,
    })
    .unwrap();
    let materializer = Materializer::new(release);
    let request = SnapshotRequest {
        cell,
        at: WorldInstant::from_seconds(1_000),
        layers: LayerMask::only(LayerKind::Geology),
        requested_detail: DetailLevel::Local,
        budget: ResourceBudget::SERVER_CEILING,
    };

    let started = Instant::now();
    for _ in 0..iterations {
        black_box(materializer.snapshot(black_box(request)).unwrap());
    }
    let elapsed = started.elapsed();
    let nanos_per_query = elapsed.as_nanos() / iterations;
    println!(
        "{iterations} materializations in {elapsed:?}; {nanos_per_query} ns/query; 256 active patches"
    );
}
