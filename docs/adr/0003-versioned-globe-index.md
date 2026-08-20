# ADR 0003: Versioned cube-sphere logical index

- Status: accepted
- Date: 2026-08-19

## Decision

The first logical partition index has six cube faces and a 28-level quadtree path packed into 64
bits. Every artifact tags `hw-cube-sphere-v1`. Canonical geometry retains its reference frame and
does not depend on the index.

Cesium 3D Tiles, S2, H3, Web Mercator, World Partition coordinates, and local tangent frames are
adapters/indexes, not source geometry.

## Consequences

The runtime gets compact parent/child traversal and polar coverage. Cube-face distortion and
boundary rules require golden tests. A better future scheme can be added and rebuilt from retained
geometry without rewriting evidence.

The v1 point-to-cell projection uses trigonometry and is therefore an offline, pinned-compiler
operation, not a cross-platform runtime authority. Exact face, corner, pole, and level-28 boundary
cases belong in cross-target goldens; pole longitude is normalized. Releases store the resulting
cell plus the retained source geometry/reference frame, so a future fixed-point projection or new
cell scheme can be rebuilt without changing evidence.
