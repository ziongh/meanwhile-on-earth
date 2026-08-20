use core::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::DomainError;

/// A stable 128-bit identity serialized as hexadecimal, never as a JSON number.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StableId(u128);

impl StableId {
    pub const fn from_u128(value: u128) -> Self {
        Self(value)
    }

    pub const fn as_u128(self) -> u128 {
        self.0
    }

    pub const fn to_be_bytes(self) -> [u8; 16] {
        self.0.to_be_bytes()
    }
}

impl fmt::Display for StableId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:032x}", self.0)
    }
}

impl fmt::Debug for StableId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "StableId({self})")
    }
}

impl FromStr for StableId {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 32 {
            return Err(DomainError::InvalidStableId);
        }
        u128::from_str_radix(value, 16)
            .map(Self)
            .map_err(|_| DomainError::InvalidStableId)
    }
}

impl Serialize for StableId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for StableId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(de::Error::custom)
    }
}

/// A BLAKE3 content digest used to pin immutable semantic artifacts.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    pub const ZERO: Self = Self([0; 32]);

    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for ContentHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ContentHash({self})")
    }
}

impl FromStr for ContentHash {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 64 {
            return Err(DomainError::InvalidContentHash);
        }
        let mut bytes = [0_u8; 32];
        for (index, byte) in bytes.iter_mut().enumerate() {
            let offset = index * 2;
            *byte = u8::from_str_radix(&value[offset..offset + 2], 16)
                .map_err(|_| DomainError::InvalidContentHash)?;
        }
        Ok(Self(bytes))
    }
}

impl Serialize for ContentHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ContentHash {
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
    fn ids_round_trip_as_strings() {
        let id = StableId::from_u128(u128::MAX);
        assert_eq!(id.to_string().parse::<StableId>().unwrap(), id);
        assert_eq!(
            serde_json::from_str::<StableId>(&serde_json::to_string(&id).unwrap()).unwrap(),
            id
        );
    }

    #[test]
    fn hashes_round_trip_as_strings() {
        let hash = ContentHash::from_bytes([0xab; 32]);
        assert_eq!(hash.to_string().parse::<ContentHash>().unwrap(), hash);
    }
}
