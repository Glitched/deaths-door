//! Unit tests for scene effects: sound pairing and audio-length-driven duration.

use deaths_door::effects::paired_sound;
use deaths_door::lighting::LightingScene;
use deaths_door::sound::{self, SoundName};

#[test]
fn game_scenes_have_paired_sounds_and_utility_scenes_do_not() {
    assert_eq!(paired_sound(LightingScene::Death), Some(SoundName::Death));
    assert_eq!(paired_sound(LightingScene::Drama), Some(SoundName::Drama));
    assert_eq!(
        paired_sound(LightingScene::Goodnight),
        Some(SoundName::MusicBox)
    );
    assert_eq!(
        paired_sound(LightingScene::Morning),
        Some(SoundName::Rooster)
    );
    assert_eq!(
        paired_sound(LightingScene::Reveal),
        Some(SoundName::Drumroll)
    );

    // Utility scenes are lights-only.
    assert_eq!(paired_sound(LightingScene::Blackout), None);
    assert_eq!(paired_sound(LightingScene::Fog), None);
    assert_eq!(paired_sound(LightingScene::Spotlight), None);
}

#[test]
fn paired_sound_durations_are_readable_and_sane() {
    // Every paired sound's audio file must exist and decode, since its length
    // drives the effect length.
    for scene in LightingScene::ALL {
        let Some(sound) = paired_sound(scene) else {
            continue;
        };
        let duration = sound::duration(sound)
            .unwrap_or_else(|| panic!("could not read duration for {sound:?}"));
        assert!(
            duration.as_millis() >= 500,
            "{sound:?} suspiciously short: {duration:?}"
        );
        assert!(
            duration.as_secs() <= 30,
            "{sound:?} suspiciously long: {duration:?}"
        );
    }
}
