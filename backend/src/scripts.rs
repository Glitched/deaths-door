//! Script registry. Character/traveler/night-step data is dumped from the
//! Python source at build time and embedded into the binary, so the Rust port
//! is byte-for-byte faithful to the original game definitions.

use std::collections::HashMap;

use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::script::Script;
use crate::script_name::ScriptName;

const DATA_JSON: &str = include_str!("../data/botc_data.json");

#[derive(Debug, Deserialize)]
struct GameData {
    scripts: HashMap<String, Script>,
}

static DATA: Lazy<GameData> =
    Lazy::new(|| serde_json::from_str(DATA_JSON).expect("embedded botc_data.json must be valid"));

/// Return the [`Script`] for a given name value, or `None` if it has no data.
///
/// Mirrors Python's `get_script_by_name`: only Trouble Brewing is wired up.
pub fn get_script_by_name(name: &str) -> Option<&'static Script> {
    let script_name = ScriptName::from_str(name)?;
    DATA.scripts.get(script_name.value())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Forces the embedded `botc_data.json` to parse, so a corrupt data file
    /// fails the test suite instead of panicking on the first request.
    #[test]
    fn embedded_data_parses_and_is_well_formed() {
        let script =
            get_script_by_name("trouble_brewing").expect("trouble_brewing must be present");
        assert_eq!(script.characters.len(), 22);
        assert_eq!(script.travelers.len(), 5);
        assert!(!script.first_night_steps.is_empty());
        assert!(!script.other_night_steps.is_empty());
        for c in script.characters.iter().chain(&script.travelers) {
            assert!(!c.name.is_empty(), "character has empty name");
            assert!(
                !c.description.is_empty(),
                "{} has empty description",
                c.name
            );
        }
    }
}
