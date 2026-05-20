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
/// All three base editions (Trouble Brewing, Sects & Violets, Bad Moon Rising)
/// have their character pools and night orders wired up; travelers are only
/// populated for Trouble Brewing so far.
pub fn get_script_by_name(name: &str) -> Option<&'static Script> {
    let script_name = ScriptName::from_str(name)?;
    DATA.scripts.get(script_name.value())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Forces the embedded `botc_data.json` to parse, so a corrupt data file
    /// fails the test suite instead of panicking on the first request, and
    /// checks the expected role counts for each wired-up edition.
    #[test]
    fn embedded_data_parses_and_is_well_formed() {
        // (script id, expected character count, expected traveler count)
        let expected = [
            ("trouble_brewing", 22, 5),
            ("sects_and_violets", 25, 0),
            ("bad_moon_rising", 25, 0),
        ];
        for (name, char_count, traveler_count) in expected {
            let script =
                get_script_by_name(name).unwrap_or_else(|| panic!("{name} must be present"));
            assert_eq!(
                script.characters.len(),
                char_count,
                "{name} character count"
            );
            assert_eq!(
                script.travelers.len(),
                traveler_count,
                "{name} traveler count"
            );
            // Every edition begins at Dusk and ends at Dawn for both night orders.
            for steps in [&script.first_night_steps, &script.other_night_steps] {
                assert_eq!(steps.first().map(|s| s.name.as_str()), Some("Dusk"));
                assert_eq!(steps.last().map(|s| s.name.as_str()), Some("Dawn"));
            }
            for c in script.characters.iter().chain(&script.travelers) {
                assert!(!c.name.is_empty(), "{name}: character has empty name");
                assert!(
                    !c.description.is_empty(),
                    "{name}: {} has empty description",
                    c.name
                );
            }
        }
    }
}
