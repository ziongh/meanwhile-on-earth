use serde::{Deserialize, Serialize};

use crate::{ContentHash, DomainError, StableId};

/// How strongly a semantic value is tied to evidence versus reconstruction.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpistemicClass {
    DirectObservation,
    ReportedPrimarySource,
    ScholarlyInference,
    ModelDerived,
    PlausibleReconstruction,
    ArtisticAmbient,
}

/// An optional numeric assessment with its method kept explicit.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfidenceBand {
    pub lower_ppm: u32,
    pub preferred_ppm: u32,
    pub upper_ppm: u32,
    pub assessment_method: String,
}

impl ConfidenceBand {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.lower_ppm > self.preferred_ppm
            || self.preferred_ppm > self.upper_ppm
            || self.upper_ppm > 1_000_000
        {
            return Err(DomainError::InvalidConfidenceBand);
        }
        Ok(())
    }
}

/// An exact immutable claim revision included in a corpus release.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimRef {
    pub claim_id: StableId,
    pub revision: u32,
    pub content_hash: ContentHash,
}

/// How a citation relates to a claim.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRole {
    Supports,
    Contradicts,
    Contextualizes,
}

/// An exact immutable citation/source revision.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRef {
    pub citation_id: StableId,
    pub source_revision_id: StableId,
    pub content_hash: ContentHash,
    pub role: EvidenceRole,
}
