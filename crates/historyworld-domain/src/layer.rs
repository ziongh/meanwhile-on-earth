use serde::{Deserialize, Serialize};

use crate::DomainError;

/// Independently streamable semantic layers.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum LayerKind {
    Geology = 0,
    Hydrology = 1,
    Climate = 2,
    Biome = 3,
    Population = 4,
    Settlement = 5,
    Political = 6,
    Event = 7,
}

impl LayerKind {
    pub const ALL: [Self; 8] = [
        Self::Geology,
        Self::Hydrology,
        Self::Climate,
        Self::Biome,
        Self::Population,
        Self::Settlement,
        Self::Political,
        Self::Event,
    ];

    pub const fn bit(self) -> u16 {
        1_u16 << (self as u8)
    }
}

/// Compact selection of independently streamable layers.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LayerMask(u16);

impl LayerMask {
    pub const NONE: Self = Self(0);
    pub const ALL: Self = Self((1_u16 << LayerKind::ALL.len()) - 1);

    pub const fn only(layer: LayerKind) -> Self {
        Self(layer.bit())
    }

    pub const fn contains(self, layer: LayerKind) -> bool {
        self.0 & layer.bit() != 0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub fn iter(self) -> impl Iterator<Item = LayerKind> {
        LayerKind::ALL
            .into_iter()
            .filter(move |layer| self.contains(*layer))
    }
}

/// Sparse, quantized semantic surface state. `None` means unknown, never zero.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SurfaceState {
    pub mean_elevation_mm: Option<i64>,
    pub relief_mm: Option<u64>,
    pub land_fraction_ppm: Option<u32>,
    pub water_fraction_ppm: Option<u32>,
    pub vegetation_fraction_ppm: Option<u32>,
    pub mean_temperature_millicelsius: Option<i32>,
    pub annual_precipitation_mm: Option<u32>,
}

impl SurfaceState {
    pub fn validate(&self) -> Result<(), DomainError> {
        for value in [
            self.land_fraction_ppm,
            self.water_fraction_ppm,
            self.vegetation_fraction_ppm,
        ]
        .into_iter()
        .flatten()
        {
            if value > 1_000_000 {
                return Err(DomainError::InvalidPartsPerMillion);
            }
        }
        Ok(())
    }
}
