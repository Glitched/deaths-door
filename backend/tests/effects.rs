//! Unit tests for scene effects: sound pairing, audio-length-driven duration,
//! and fixture cleanup on scene changes.

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
    assert_eq!(paired_sound(LightingScene::Alarm), Some(SoundName::Alarm));
    assert_eq!(
        paired_sound(LightingScene::SadTrumpet),
        Some(SoundName::SadTrumpet)
    );
    assert_eq!(
        paired_sound(LightingScene::Wilhelm),
        Some(SoundName::Wilhelm)
    );

    // Utility scenes are lights-only.
    assert_eq!(paired_sound(LightingScene::Blackout), None);
    assert_eq!(paired_sound(LightingScene::Fog), None);
    assert_eq!(paired_sound(LightingScene::Spotlight), None);
}

/// A superseded scene never runs its fog-off cue, so triggering a new scene
/// must reset the fog machine itself — otherwise fog runs until the next
/// scene that happens to touch it.
#[tokio::test]
async fn a_new_trigger_resets_orphaned_fog() {
    use std::sync::Arc;

    use deaths_door::effects::EffectsEngine;
    use deaths_door::event_store::EventStore;
    use deaths_door::game_manager::GameManager;
    use deaths_door::lighting::LightingManager;

    const FOG_CHANNEL: usize = 22; // DMX channel 23

    let lighting = Arc::new(LightingManager::new());
    let manager = Arc::new(GameManager::new(EventStore::in_memory().unwrap()));
    let engine = Arc::new(EffectsEngine::new(Arc::clone(&lighting), manager));

    // Simulate a scene that was cut off mid-fog-burst.
    lighting.set_fog(200);
    assert_eq!(lighting.universe_snapshot()[FOG_CHANNEL], 200);

    // Morning has no fog cues of its own; the trigger itself must clear it.
    engine.trigger(LightingScene::Morning, true).await;
    assert_eq!(lighting.universe_snapshot()[FOG_CHANNEL], 0);
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
