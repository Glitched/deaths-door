//! A character in a script, plus its API output model.

use serde::{Deserialize, Serialize};

use crate::alignment::Alignment;
use crate::character_type::CharacterType;
use crate::status_effect::StatusEffectOut;

/// A character with only the fields meant to be sent to the client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CharacterOut {
    pub name: String,
    pub description: String,
    pub icon_path: String,
    pub alignment: Alignment,
    pub category: CharacterType,
}

/// A character in the script. The icon filename is derived on demand via
/// [`Character::get_icon_path`]; any `icon_path`/`changes` keys in the source
/// data are ignored (serde ignores unknown fields by default).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Character {
    pub name: String,
    pub description: String,
    pub category: CharacterType,
    pub alignment: Alignment,
    /// Names of the status effects this character can introduce.
    #[serde(default)]
    pub status_effects: Vec<String>,
}

impl Character {
    fn normalize(name: &str) -> String {
        name.to_lowercase().trim().to_string()
    }

    /// Check if the character matches the given name (case-insensitive, trimmed).
    pub fn is_named(&self, name: &str) -> bool {
        Self::normalize(&self.name) == Self::normalize(name)
    }

    /// The character's icon filename, matching Python's `get_icon_path`.
    pub fn get_icon_path(&self) -> String {
        format!("{}.png", self.name.to_lowercase().replace(' ', ""))
    }

    /// The character's status effects as API output models.
    pub fn get_status_effects_out(&self) -> Vec<StatusEffectOut> {
        self.status_effects
            .iter()
            .map(|effect| StatusEffectOut {
                name: effect.clone(),
                character_name: self.name.clone(),
            })
            .collect()
    }

    /// Convert the character to its API output model.
    pub fn to_out(&self) -> CharacterOut {
        CharacterOut {
            name: self.name.clone(),
            description: self.description.clone(),
            icon_path: self.get_icon_path(),
            alignment: self.alignment,
            category: self.category,
        }
    }
}
