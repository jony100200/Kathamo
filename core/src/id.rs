//! Deterministic stable element identity for Kathamo retained UI elements.
//!
//! Provides deterministic 64-bit IDs generated via FNV-1a hashing of semantic
//! paths and keys. Guaranteed stable across frames, tab switches, and redraws.

use std::fmt;

/// Stable 64-bit element identifier.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ElementId(pub u64);

impl ElementId {
    /// The root or null element ID.
    pub const NULL: Self = Self(0);

    /// Creates a stable ElementId by hashing a string path or key.
    pub fn from_str(key: &str) -> Self {
        Self(fnv1a_hash(key.as_bytes()))
    }

    /// Creates an ElementId from raw 64-bit integer.
    pub const fn from_u64(val: u64) -> Self {
        Self(val)
    }

    /// Derives a child ID scoped under this parent ID.
    pub fn child(&self, child_key: &str) -> Self {
        let mut hash = self.0;
        for &byte in child_key.as_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        Self(hash)
    }

    /// Derives a child ID with an integer index (useful for virtualized lists).
    pub fn with_index(&self, index: usize) -> Self {
        let mut hash = self.0;
        for &byte in &index.to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        Self(hash)
    }

    /// Returns the raw 64-bit value.
    pub const fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Debug for ElementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ElementId(0x{:016x})", self.0)
    }
}

impl fmt::Display for ElementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#0x{:016x}", self.0)
    }
}

impl From<&str> for ElementId {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

impl From<u64> for ElementId {
    fn from(val: u64) -> Self {
        Self(val)
    }
}

/// 64-bit FNV-1a non-cryptographic hash function.
#[inline]
fn fnv1a_hash(bytes: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET_BASIS;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_id_stability() {
        let id1 = ElementId::from_str("workspace.left_panel.layers");
        let id2 = ElementId::from_str("workspace.left_panel.layers");
        assert_eq!(id1, id2);
        assert_ne!(id1, ElementId::NULL);
    }

    #[test]
    fn test_child_derivation_is_unique_and_scoped() {
        let parent = ElementId::from_str("parent");
        let child1 = parent.child("child1");
        let child2 = parent.child("child2");
        let child1_again = parent.child("child1");

        assert_eq!(child1, child1_again);
        assert_ne!(child1, child2);
        assert_ne!(child1, parent);
    }

    #[test]
    fn test_indexed_children() {
        let parent = ElementId::from_str("list");
        let item0 = parent.with_index(0);
        let item1 = parent.with_index(1);
        let item0_again = parent.with_index(0);

        assert_eq!(item0, item0_again);
        assert_ne!(item0, item1);
    }
}
