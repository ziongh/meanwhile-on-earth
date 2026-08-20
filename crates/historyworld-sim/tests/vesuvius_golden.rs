use historyworld_content::CompiledRelease;
use historyworld_domain::{
    ContentHash, DetailLevel, LayerMask, ResourceBudget, WorldCell, WorldInstant,
};
use historyworld_sim::{Materializer, SnapshotRequest};

const FIXTURE: &str = include_str!("../../../data/fixtures/vesuvius-architecture-demo.json");

#[test]
fn bundled_release_and_eruption_boundary_are_compatibility_golden() {
    let release = CompiledRelease::from_json(FIXTURE).unwrap();
    assert_eq!(
        release.manifest().release_id,
        "ed48622f3868afbda2108ad0a37d98b00735c41e4ff3f8cfbb0ab7e47152f76b"
            .parse::<ContentHash>()
            .unwrap()
    );
    let materializer = Materializer::new(release);
    let cell = "0:14:ee085cd".parse::<WorldCell>().unwrap();
    let snapshot = |year, month, day| {
        materializer
            .snapshot(SnapshotRequest {
                cell,
                at: WorldInstant::from_ymd_hms(year, month, day, 0, 0, 0).unwrap(),
                layers: LayerMask::ALL,
                requested_detail: DetailLevel::Local,
                budget: ResourceBudget::INTERACTIVE,
            })
            .unwrap()
    };

    let before = snapshot(79, 8, 23);
    let same_segment = snapshot(79, 8, 22);
    let after = snapshot(79, 8, 25);
    assert_eq!(before.surface.mean_elevation_mm, Some(1_800_000));
    assert_eq!(after.surface.mean_elevation_mm, Some(1_350_000));
    assert_eq!(before.surface.vegetation_fraction_ppm, Some(650_000));
    assert_eq!(after.surface.vegetation_fraction_ppm, Some(50_000));
    assert_eq!(before.semantic_state_hash, same_segment.semantic_state_hash);
    assert_eq!(
        before.semantic_state_hash,
        "583547338f6ffae0cf6cd21bd6565bf9f9a45a126172b45c082aeb887d7b3997"
            .parse::<ContentHash>()
            .unwrap()
    );
    assert_eq!(
        after.semantic_state_hash,
        "45b2ccd139baede62ca99dd48557d73d0b5ac1222475b657e487b3a589b1ac7d"
            .parse::<ContentHash>()
            .unwrap()
    );
}
