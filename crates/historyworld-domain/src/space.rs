use core::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{ContentHash, DomainError, WorldInstant};

const FACE_SHIFT: u32 = 61;
const LEVEL_SHIFT: u32 = 56;
const LEVEL_MASK: u64 = 0x1f;
const PATH_MASK: u64 = (1_u64 << LEVEL_SHIFT) - 1;

/// A centimeter-scale WGS84-style geodetic point without floating-point drift.
///
/// The owning artifact must also pin a [`SpatialReference`]. This value is an
/// interchange coordinate, never an Unreal `FVector` or origin-relative point.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct GeoPointE7 {
    pub latitude_e7: i32,
    pub longitude_e7: i32,
    pub height_mm: i64,
}

impl<'de> Deserialize<'de> for GeoPointE7 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct WirePoint {
            latitude_e7: i32,
            longitude_e7: i32,
            height_mm: i64,
        }

        let wire = WirePoint::deserialize(deserializer)?;
        Self::new(wire.latitude_e7, wire.longitude_e7, wire.height_mm).map_err(de::Error::custom)
    }
}

impl GeoPointE7 {
    pub fn new(latitude_e7: i32, longitude_e7: i32, height_mm: i64) -> Result<Self, DomainError> {
        if !(-900_000_000..=900_000_000).contains(&latitude_e7) {
            return Err(DomainError::InvalidLatitude);
        }
        if !(-1_800_000_000..1_800_000_000).contains(&longitude_e7) {
            return Err(DomainError::InvalidLongitude);
        }
        // All longitudes describe the same physical point at either pole. A
        // single representation prevents floating residuals from selecting
        // different level-28 paths on different longitudes.
        let longitude_e7 = if latitude_e7.unsigned_abs() == 900_000_000 {
            0
        } else {
            longitude_e7
        };
        Ok(Self {
            latitude_e7,
            longitude_e7,
            height_mm,
        })
    }
}

/// Versioned coordinate context for canonical geometry.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SpatialReference {
    pub authority: String,
    pub code: String,
    pub revision: String,
    pub ellipsoid: String,
    pub vertical_datum: String,
    pub coordinate_epoch: Option<WorldInstant>,
    pub reconstruction_model: Option<ContentHash>,
}

/// A cell in HistoryWorld cube-sphere scheme version 1.
///
/// The packed layout is `[face:3][level:5][quadtree path:56]`. It has six
/// roots and 28 levels, covers poles without Web Mercator, and remains an index
/// only: canonical geometry never depends on this particular cell scheme.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WorldCell(u64);

impl WorldCell {
    pub const MAX_LEVEL: u8 = 28;

    pub fn new(face: u8, level: u8, path: u64) -> Result<Self, DomainError> {
        if face > 5 {
            return Err(DomainError::InvalidCellFace);
        }
        if level > Self::MAX_LEVEL {
            return Err(DomainError::InvalidCellLevel(Self::MAX_LEVEL));
        }
        let path_bits = u32::from(level) * 2;
        let valid_path = if path_bits == 0 {
            path == 0
        } else {
            path < (1_u64 << path_bits)
        };
        if !valid_path {
            return Err(DomainError::InvalidCellPath);
        }
        Ok(Self(
            (u64::from(face) << FACE_SHIFT) | (u64::from(level) << LEVEL_SHIFT) | path,
        ))
    }

    pub fn from_raw(raw: u64) -> Result<Self, DomainError> {
        let face = u8::try_from(raw >> FACE_SHIFT).map_err(|_| DomainError::InvalidCellEncoding)?;
        let level = u8::try_from((raw >> LEVEL_SHIFT) & LEVEL_MASK)
            .map_err(|_| DomainError::InvalidCellEncoding)?;
        let cell = Self::new(face, level, raw & PATH_MASK)?;
        if cell.0 != raw {
            return Err(DomainError::InvalidCellEncoding);
        }
        Ok(cell)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub const fn face(self) -> u8 {
        (self.0 >> FACE_SHIFT) as u8
    }

    pub const fn level(self) -> u8 {
        ((self.0 >> LEVEL_SHIFT) & LEVEL_MASK) as u8
    }

    pub const fn path(self) -> u64 {
        self.0 & PATH_MASK
    }

    pub fn parent(self) -> Result<Self, DomainError> {
        if self.level() == 0 {
            return Err(DomainError::CellHasNoParent);
        }
        Self::new(self.face(), self.level() - 1, self.path() >> 2)
    }

    pub fn child(self, quadrant: u8) -> Result<Self, DomainError> {
        if self.level() == Self::MAX_LEVEL {
            return Err(DomainError::CellHasNoChildren);
        }
        if quadrant > 3 {
            return Err(DomainError::InvalidCellPath);
        }
        Self::new(
            self.face(),
            self.level() + 1,
            (self.path() << 2) | u64::from(quadrant),
        )
    }

    pub fn ancestor(self, level: u8) -> Result<Self, DomainError> {
        if level > self.level() {
            return Err(DomainError::InvalidCellLevel(self.level()));
        }
        let shift = u32::from(self.level() - level) * 2;
        Self::new(self.face(), level, self.path() >> shift)
    }

    pub fn ancestors_root_first(self) -> impl Iterator<Item = Self> {
        (0..=self.level()).map(move |level| {
            self.ancestor(level)
                .expect("levels produced by this iterator are valid ancestors")
        })
    }

    /// Converts an authoring point into the version-1 cube-sphere index.
    ///
    /// Boundary assignment is defined by this reference implementation. Source
    /// geometry is retained separately so a future cell scheme can be rebuilt.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub fn from_point(point: GeoPointE7, level: u8) -> Result<Self, DomainError> {
        if level > Self::MAX_LEVEL {
            return Err(DomainError::InvalidCellLevel(Self::MAX_LEVEL));
        }
        let latitude = f64::from(point.latitude_e7) * 1e-7_f64.to_radians();
        let longitude = f64::from(point.longitude_e7) * 1e-7_f64.to_radians();
        let (sin_latitude, cos_latitude) = latitude.sin_cos();
        let (sin_longitude, cos_longitude) = longitude.sin_cos();
        let x = cos_latitude * cos_longitude;
        let y = cos_latitude * sin_longitude;
        let z = sin_latitude;
        let ax = x.abs();
        let ay = y.abs();
        let az = z.abs();

        let (face, u, v) = if ax >= ay && ax >= az {
            if x >= 0.0 {
                (0, y / ax, z / ax)
            } else {
                (1, -y / ax, z / ax)
            }
        } else if ay >= az {
            if y >= 0.0 {
                (2, -x / ay, z / ay)
            } else {
                (3, x / ay, z / ay)
            }
        } else if z >= 0.0 {
            (4, y / az, -x / az)
        } else {
            (5, y / az, x / az)
        };

        let width = 1_u32 << level;
        let index = |coordinate: f64| -> u32 {
            (((coordinate + 1.0) * 0.5 * f64::from(width)).floor() as i64)
                .clamp(0, i64::from(width) - 1) as u32
        };
        let (cell_x, cell_y) = (index(u), index(v));
        let mut path = 0_u64;
        for bit_index in (0..level).rev() {
            let x_bit = (cell_x >> bit_index) & 1;
            let y_bit = (cell_y >> bit_index) & 1;
            path = (path << 2) | u64::from((y_bit << 1) | x_bit);
        }
        Self::new(face, level, path)
    }
}

impl fmt::Display for WorldCell {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}:{:x}",
            self.face(),
            self.level(),
            self.path()
        )
    }
}

impl fmt::Debug for WorldCell {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "WorldCell({self})")
    }
}

impl FromStr for WorldCell {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut parts = value.split(':');
        let face = parts
            .next()
            .ok_or(DomainError::InvalidCellEncoding)?
            .parse::<u8>()
            .map_err(|_| DomainError::InvalidCellEncoding)?;
        let level = parts
            .next()
            .ok_or(DomainError::InvalidCellEncoding)?
            .parse::<u8>()
            .map_err(|_| DomainError::InvalidCellEncoding)?;
        let path = u64::from_str_radix(parts.next().ok_or(DomainError::InvalidCellEncoding)?, 16)
            .map_err(|_| DomainError::InvalidCellEncoding)?;
        if parts.next().is_some() {
            return Err(DomainError::InvalidCellEncoding);
        }
        Self::new(face, level, path)
    }
}

impl Serialize for WorldCell {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for WorldCell {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_hierarchy_round_trips() {
        let root = WorldCell::new(4, 0, 0).unwrap();
        let grandchild = root.child(2).unwrap().child(3).unwrap();
        assert_eq!(grandchild.parent().unwrap().parent().unwrap(), root);
        assert_eq!(
            grandchild.ancestors_root_first().collect::<Vec<_>>()[0],
            root
        );
        assert_eq!(
            grandchild.to_string().parse::<WorldCell>().unwrap(),
            grandchild
        );
        assert_eq!(WorldCell::from_raw(grandchild.raw()).unwrap(), grandchild);
    }

    #[test]
    fn point_assignment_handles_poles_and_antimeridian() {
        let north = GeoPointE7::new(900_000_000, 0, 0).unwrap();
        let north_other_longitude = GeoPointE7::new(900_000_000, 900_000_000, 0).unwrap();
        let west_of_dateline = GeoPointE7::new(0, -1_799_999_999, 0).unwrap();
        let east_of_dateline = GeoPointE7::new(0, 1_799_999_999, 0).unwrap();
        assert_eq!(WorldCell::from_point(north, 10).unwrap().face(), 4);
        assert_eq!(north, north_other_longitude);
        assert_eq!(
            WorldCell::from_point(north, WorldCell::MAX_LEVEL).unwrap(),
            WorldCell::from_point(north_other_longitude, WorldCell::MAX_LEVEL).unwrap()
        );
        assert_eq!(
            WorldCell::from_point(west_of_dateline, 10).unwrap().face(),
            WorldCell::from_point(east_of_dateline, 10).unwrap().face()
        );
    }

    #[test]
    fn json_uses_portable_string_encoding() {
        let cell = WorldCell::new(5, 28, (1_u64 << 56) - 1).unwrap();
        let json = serde_json::to_string(&cell).unwrap();
        assert_eq!(serde_json::from_str::<WorldCell>(&json).unwrap(), cell);
    }
}
