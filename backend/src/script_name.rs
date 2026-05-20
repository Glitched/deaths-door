//! The name of a script (edition).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptName {
    TroubleBrewing,
    SectsAndViolets,
    BadMoonRising,
}

impl ScriptName {
    pub const ALL: [ScriptName; 3] = [
        ScriptName::TroubleBrewing,
        ScriptName::SectsAndViolets,
        ScriptName::BadMoonRising,
    ];

    /// Machine-readable value (e.g. `trouble_brewing`).
    pub fn value(&self) -> &'static str {
        match self {
            ScriptName::TroubleBrewing => "trouble_brewing",
            ScriptName::SectsAndViolets => "sects_and_violets",
            ScriptName::BadMoonRising => "bad_moon_rising",
        }
    }

    /// Human-readable display name.
    pub fn display(&self) -> &'static str {
        match self {
            ScriptName::TroubleBrewing => "Trouble Brewing",
            ScriptName::SectsAndViolets => "Sects and Violets",
            ScriptName::BadMoonRising => "Bad Moon Rising",
        }
    }

    /// Parse from a value string, case-insensitively.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(name: &str) -> Option<ScriptName> {
        let lower = name.to_lowercase();
        ScriptName::ALL.into_iter().find(|s| s.value() == lower)
    }
}
