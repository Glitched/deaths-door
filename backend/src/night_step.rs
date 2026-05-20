//! A step in the night phase for the storyteller to follow.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NightStep {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub always_show: bool,
    #[serde(default)]
    pub show_when_dead: bool,
}
