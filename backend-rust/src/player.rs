//! Player API output model.

use serde::{Deserialize, Serialize};

use crate::alignment::Alignment;
use crate::character::CharacterOut;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PlayerOut {
    pub name: String,
    pub character: CharacterOut,
    pub alignment: Alignment,
    pub is_alive: bool,
    pub has_used_dead_vote: bool,
    pub status_effects: Vec<String>,
}
