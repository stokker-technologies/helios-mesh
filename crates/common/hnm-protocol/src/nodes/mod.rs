/* ***
 *
 * Helios Node Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Stable node definitions shared by firmware and host clients.

/* === Modules === */

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports

// Third Party Imports

// Local Imports

/* === Constants === */

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeDefinition {
    pub identifier: &'static str,

    pub label: &'static str,

    pub description: &'static str,
}

/* === Enums === */

#[hnm_macros::node_catalog]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NodeType {
    Sidecar,

    Gateway,

    Rotator,
}

/* === Traits === */

/* === Implementations === */

impl serde::Serialize for NodeType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.definition().identifier)
    }
}

impl<'de> serde::Deserialize<'de> for NodeType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Identifier;
        impl serde::de::Visitor<'_> for Identifier {
            type Value = NodeType;
            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str("a registered node identifier")
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<NodeType, E> {
                NodeType::from_identifier(value).ok_or_else(|| E::custom("unknown node identifier"))
            }
        }
        deserializer.deserialize_str(Identifier)
    }
}

/* === Public Functions === */

/* === Private Functions === */

/* === Tests === */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_identity_position() {
        for kind in NodeType::ALL {
            let mut bytes = [0; 64];
            let encoded = postcard::to_slice(&kind, &mut bytes).unwrap();
            assert!(encoded.windows(kind.definition().identifier.len()).any(|window| window == kind.definition().identifier.as_bytes()));
            assert_eq!(postcard::from_bytes::<NodeType>(encoded).unwrap(), kind);
            assert_eq!(NodeType::from_identifier(kind.definition().identifier), Some(kind));
            assert_eq!(NodeType::from_identifier(kind.definition().label), None);
        }
        let mut bytes = [0; 64];
        let unknown = postcard::to_slice("fork.unknown", &mut bytes).unwrap();
        assert!(postcard::from_bytes::<NodeType>(unknown).is_err());
    }
}
