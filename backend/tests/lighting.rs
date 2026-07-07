//! Tests for the DMX shadow universe: fixture channel math, blackout, fog,
//! position handling, and calibration persistence. No hardware is required —
//! the shadow universe updates even when no transmitter is running.

use deaths_door::lighting::{colors, LightingManager};

// Fixture layout (see lighting.rs): 11-channel heads at DMX 1 and 12, fog at 23.
// `universe_snapshot()[i]` is DMX channel i+1.
const L1: usize = 0; // light 1 base index
const L2: usize = 11; // light 2 base index
const FOG: usize = 22; // fog machine index
const PAN: usize = 0;
const PAN_FINE: usize = 1;
const TILT: usize = 2;
const TILT_FINE: usize = 3;
const COLOR: usize = 4;
const STROBE: usize = 7;
const DIMMER: usize = 8;

#[test]
fn set_heads_writes_color_dimmer_strobe_for_both_heads() {
    let m = LightingManager::new();
    m.set_heads((colors::RED, colors::BLUE), 200, 50);

    let u = m.universe_snapshot();
    assert_eq!(u[L1 + COLOR], colors::RED as u8);
    assert_eq!(u[L2 + COLOR], colors::BLUE as u8);
    for base in [L1, L2] {
        assert_eq!(u[base + DIMMER], 200);
        assert_eq!(u[base + STROBE], 50);
    }
}

#[test]
fn blackout_kills_dimmers_and_fog_but_keeps_colors() {
    let m = LightingManager::new();
    m.set_heads((colors::RED, colors::RED), 255, 0);
    m.set_fog(200);

    m.blackout();
    let u = m.universe_snapshot();
    assert_eq!(u[L1 + DIMMER], 0);
    assert_eq!(u[L2 + DIMMER], 0);
    assert_eq!(u[FOG], 0);
    // Colors survive so a scene can fade back in from where it was.
    assert_eq!(u[L1 + COLOR], colors::RED as u8);
}

#[test]
fn set_position_is_coarse_only_and_resets_fine_channels() {
    let m = LightingManager::new();
    // Leftover fine-channel values from manual fiddling...
    m.set_channel(1, (PAN_FINE + 1) as i64, 99);
    m.set_channel(1, (TILT_FINE + 1) as i64, 99);

    // ...are cleared by a fine-position move, not mirrored from coarse.
    m.set_position(1, 100, 50, true);
    let u = m.universe_snapshot();
    assert_eq!(u[L1 + PAN], 100);
    assert_eq!(u[L1 + TILT], 50);
    assert_eq!(u[L1 + PAN_FINE], 0);
    assert_eq!(u[L1 + TILT_FINE], 0);

    // With fine=false the fine channels are left untouched.
    m.set_channel(1, (PAN_FINE + 1) as i64, 99);
    m.set_position(1, 10, 20, false);
    assert_eq!(m.universe_snapshot()[L1 + PAN_FINE], 99);
}

#[test]
fn set_channel_maps_fixtures_validates_ranges_and_clamps() {
    let m = LightingManager::new();

    m.set_channel(1, 1, 10);
    m.set_channel(2, 11, 42);
    m.set_channel(3, 1, 123); // the fog machine is addressable too
    let u = m.universe_snapshot();
    assert_eq!(u[L1], 10);
    assert_eq!(u[L2 + 10], 42);
    assert_eq!(u[FOG], 123);

    // Out-of-range channels and unknown fixtures are ignored.
    let before = m.universe_snapshot();
    m.set_channel(1, 12, 99);
    m.set_channel(3, 2, 99);
    m.set_channel(9, 1, 99);
    assert_eq!(m.universe_snapshot(), before);

    // Values clamp to the DMX 0-255 range.
    m.set_channel(1, 1, 999);
    assert_eq!(m.universe_snapshot()[L1], 255);
    m.set_channel(1, 1, -5);
    assert_eq!(m.universe_snapshot()[L1], 0);
}

#[test]
fn spotlight_uses_saved_positions_and_calibration_persists() {
    // Point calibration at a scratch file so the test never touches the repo.
    let file = std::env::temp_dir().join(format!("dd_positions_{}.json", std::process::id()));
    std::env::set_var("LIGHTING_POSITIONS_PATH", &file);

    let m = LightingManager::new();

    // No saved position -> no output change.
    m.spotlight_player(3, 255, 1);
    assert_eq!(m.universe_snapshot(), [0u8; 512]);

    m.save_player_position(3, 111, 55);
    m.spotlight_player(3, 200, 2);
    let u = m.universe_snapshot();
    assert_eq!(u[L2 + PAN], 111);
    assert_eq!(u[L2 + TILT], 55);
    assert_eq!(u[L2 + DIMMER], 200);
    assert_eq!(u[L2 + COLOR], colors::WHITE as u8);
    assert_eq!(u[L2 + STROBE], 0);

    // A fresh manager reloads the calibration from disk.
    let reloaded = LightingManager::new();
    assert!(reloaded.has_position(3));
    assert_eq!(reloaded.get_all_positions()[&3].pan, 111);

    std::env::remove_var("LIGHTING_POSITIONS_PATH");
    let _ = std::fs::remove_file(&file);
}
