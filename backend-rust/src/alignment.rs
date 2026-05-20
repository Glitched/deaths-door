//! The alignment of a role.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Alignment {
    Good,
    Evil,
    Unknown,
}

impl Alignment {
    /// The wire string value, matching the Python `Alignment` enum.
    pub fn as_str(&self) -> &'static str {
        match self {
            Alignment::Good => "good",
            Alignment::Evil => "evil",
            Alignment::Unknown => "unknown",
        }
    }

    /// Parse from a string value, case-insensitively.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(name: &str) -> Option<Alignment> {
        match name.to_lowercase().as_str() {
            "good" => Some(Alignment::Good),
            "evil" => Some(Alignment::Evil),
            "unknown" => Some(Alignment::Unknown),
            _ => None,
        }
    }
}
