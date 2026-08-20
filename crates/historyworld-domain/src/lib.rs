//! Stable, engine-neutral domain types for HistoryWorld.
//!
//! These types deliberately avoid Unreal, database timestamps, platform-sized
//! integers, and authoritative floating-point state. Renderers and persistence
//! adapters translate at the edges.

mod budget;
mod id;
mod layer;
mod provenance;
mod space;
mod time;

pub use budget::{DetailLevel, ResourceBudget};
pub use id::{ContentHash, StableId};
pub use layer::{LayerKind, LayerMask, SurfaceState};
pub use provenance::{ClaimRef, ConfidenceBand, EpistemicClass, EvidenceRef, EvidenceRole};
pub use space::{GeoPointE7, SpatialReference, WorldCell};
pub use time::{TemporalQueryMode, TimeRange, UncertainTimeRange, WorldInstant};

use thiserror::Error;

/// Errors caused by invalid canonical domain values.
#[derive(Debug, Error, Clone, Eq, PartialEq)]
pub enum DomainError {
    #[error("nanoseconds must be below 1,000,000,000, got {0}")]
    InvalidNanoseconds(u32),
    #[error("invalid proleptic Gregorian date: {year}-{month:02}-{day:02}")]
    InvalidDate { year: i64, month: u8, day: u8 },
    #[error("invalid time of day: {hour:02}:{minute:02}:{second:02}")]
    InvalidTimeOfDay { hour: u8, minute: u8, second: u8 },
    #[error("time arithmetic overflowed")]
    TimeOverflow,
    #[error("a half-open time range must end after it starts")]
    InvalidTimeRange,
    #[error("uncertain temporal bounds are inconsistent")]
    InvalidTemporalUncertainty,
    #[error("latitude E7 must be between -900000000 and 900000000")]
    InvalidLatitude,
    #[error("longitude E7 must be in [-1800000000, 1800000000)")]
    InvalidLongitude,
    #[error("world cell face must be between 0 and 5")]
    InvalidCellFace,
    #[error("world cell level must be at most {0}")]
    InvalidCellLevel(u8),
    #[error("world cell path contains bits outside its level")]
    InvalidCellPath,
    #[error("world cell has no parent")]
    CellHasNoParent,
    #[error("world cell at the maximum level has no children")]
    CellHasNoChildren,
    #[error("invalid world cell encoding")]
    InvalidCellEncoding,
    #[error("invalid stable id; expected 32 lowercase or uppercase hexadecimal characters")]
    InvalidStableId,
    #[error("invalid content hash; expected 64 lowercase or uppercase hexadecimal characters")]
    InvalidContentHash,
    #[error("parts-per-million value must be at most 1,000,000")]
    InvalidPartsPerMillion,
    #[error("confidence bounds must satisfy 0 <= lower <= preferred <= upper <= 1,000,000")]
    InvalidConfidenceBand,
    #[error("resource budgets must be non-zero")]
    InvalidResourceBudget,
}
