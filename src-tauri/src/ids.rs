//! Stable typed identifiers.
//!
//! Per directive §4, Sammy uses stable typed identifiers. These newtypes wrap
//! `Uuid` so that vault ids, record ids, source ids etc. cannot be mixed up at
//! the type level. Their string forms are stable across exports and backups.
//!
//! The string form is the canonical hyphenated UUID v4. External corpus
//! packages and backups MUST preserve these strings verbatim.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Macro to declare a strongly-typed id newtype around `Uuid`.
macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Generate a fresh random id using the CSPRNG-backed UUID v4.
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Parse a stable string form. Errors are opaque to avoid leaking
            /// structure details.
            pub fn parse(s: &str) -> Result<Self, crate::error::AppError> {
                Uuid::parse_str(s)
                    .map(Self)
                    .map_err(|_| {
                        crate::error::AppError::InvalidArgument(format!(
                            "invalid {} id",
                            stringify!($name)
                        ))
                    })
            }

            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<Uuid> for $name {
            fn from(u: Uuid) -> Self {
                Self(u)
            }
        }
    };
}

id_type! {
    /// Identifies an independent vault. Each vault has its own SQLCipher
    /// database, data-encryption key, sources, conversations, etc.
    VaultId
}
id_type! {
    /// Identifies a single raw source file or manual entry inside a vault.
    SourceId
}
id_type! {
    /// Identifies a curated knowledge record.
    RecordId
}
id_type! {
    /// Identifies a conversation.
    ConversationId
}
id_type! {
    /// Identifies a single message within a conversation.
    MessageId
}
id_type! {
    /// Identifies a memory candidate awaiting review.
    MemoryCandidateId
}
id_type! {
    /// Identifies an audit event. Append-only.
    AuditEventId
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip() {
        let v = VaultId::new();
        let s = v.to_string();
        let v2 = VaultId::parse(&s).unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn ids_reject_garbage() {
        assert!(VaultId::parse("not-a-uuid").is_err());
    }

    #[test]
    fn ids_are_unique() {
        assert_ne!(RecordId::new(), RecordId::new());
    }
}
