//! The category of a role.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum CharacterType {
    Townsfolk,
    Outsider,
    Minion,
    Demon,
    Traveler,
}
