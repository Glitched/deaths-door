//! Status effects applied to characters.

use serde::{Deserialize, Serialize};

/// A single status effect applied to a character (API output model).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StatusEffectOut {
    pub name: String,
    pub character_name: String,
}
