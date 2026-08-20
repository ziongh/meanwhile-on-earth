use serde::{Deserialize, Serialize};

use crate::DomainError;

/// Semantic realization detail, independent of spatial and visual quality LODs.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum DetailLevel {
    Planet,
    Continental,
    Regional,
    Local,
    Street,
}

impl DetailLevel {
    pub const fn lower(self) -> Option<Self> {
        match self {
            Self::Planet => None,
            Self::Continental => Some(Self::Planet),
            Self::Regional => Some(Self::Continental),
            Self::Local => Some(Self::Regional),
            Self::Street => Some(Self::Local),
        }
    }
}

/// Hard request limits. Every materialization path must carry a finite budget.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResourceBudget {
    pub max_candidate_patches: u32,
    pub max_output_bytes: u32,
    pub max_generated_instances: u32,
    pub max_work_units: u64,
}

impl ResourceBudget {
    pub const INTERACTIVE: Self = Self {
        max_candidate_patches: 4_096,
        max_output_bytes: 8 * 1024 * 1024,
        max_generated_instances: 20_000,
        max_work_units: 100_000,
    };

    pub const SERVER_CEILING: Self = Self {
        max_candidate_patches: 65_536,
        max_output_bytes: 32 * 1024 * 1024,
        max_generated_instances: 100_000,
        max_work_units: 1_000_000,
    };

    pub fn new(
        max_candidate_patches: u32,
        max_output_bytes: u32,
        max_generated_instances: u32,
        max_work_units: u64,
    ) -> Result<Self, DomainError> {
        if max_candidate_patches == 0
            || max_output_bytes == 0
            || max_generated_instances == 0
            || max_work_units == 0
        {
            return Err(DomainError::InvalidResourceBudget);
        }
        Ok(Self {
            max_candidate_patches,
            max_output_bytes,
            max_generated_instances,
            max_work_units,
        })
    }

    pub fn validate(self) -> Result<(), DomainError> {
        if self.max_candidate_patches == 0
            || self.max_output_bytes == 0
            || self.max_generated_instances == 0
            || self.max_work_units == 0
        {
            return Err(DomainError::InvalidResourceBudget);
        }
        Ok(())
    }

    /// Prevents a client from raising server-configured hard limits.
    pub const fn clamp_to(self, ceiling: Self) -> Self {
        Self {
            max_candidate_patches: min_u32(
                self.max_candidate_patches,
                ceiling.max_candidate_patches,
            ),
            max_output_bytes: min_u32(self.max_output_bytes, ceiling.max_output_bytes),
            max_generated_instances: min_u32(
                self.max_generated_instances,
                ceiling.max_generated_instances,
            ),
            max_work_units: min_u64(self.max_work_units, ceiling.max_work_units),
        }
    }
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self::INTERACTIVE
    }
}

const fn min_u32(left: u32, right: u32) -> u32 {
    if left < right { left } else { right }
}

const fn min_u64(left: u64, right: u64) -> u64 {
    if left < right { left } else { right }
}
