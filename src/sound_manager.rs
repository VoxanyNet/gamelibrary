use std::collections::HashMap;

use diff::Diff;
use macroquad::{audio::{load_sound, play_sound, PlaySoundParams, Sound}, math::Vec2};
use serde::{Deserialize, Serialize};



#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SoundDetails {
    path: String,
    position: Vec2,

}

#[derive(Clone, Serialize, Deserialize)]
// we need to preload the sound cache with any sounds that we want to use BEFORE. this way we dont need to use async
pub struct SoundManager {
    #[serde(skip)]
    sound_cache: HashMap<String, Sound>,
    play_history: Vec<SoundDetails>, // history of all the sound paths we have played. this is used in the diff step to determine which new sounds need to be relayed,
    #[serde(skip)]
    listener_pos: Vec2
    
}

#[derive(Debug)]
pub struct SoundManagerDiff {
    new_sounds: Option<Vec<SoundDetails>>
}

impl SoundManager {

    pub fn new(listener_position: Vec2) -> Self {
        Self {
            sound_cache: HashMap::new(),
            play_history: Vec::new(),
            listener_pos: listener_position,
            
        }
    }

    pub async fn load_sound(&mut self, path: &str) {

        let sound = load_sound(&path).await.unwrap();

        self.sound_cache.insert(path.to_string(), sound);
    }
    pub fn play_sound(&mut self, path: &str, position: Vec2) {

        let sound = self.sound_cache.get(&path.to_string()).unwrap();

        self.play_history.push(
            SoundDetails {
                path: path.to_string(),
                position,
            }
        );

        play_sound(sound, PlaySoundParams::default());
    }
}

impl Diff for SoundManager {
    type Repr = SoundManagerDiff;

    fn diff(&self, other: &Self) -> Self::Repr {
        let mut diff = SoundManagerDiff {
            new_sounds: None,
        };

        // this is majorly stupid but i think it gets optimized well by the compiler
        let mut new_entry_indices: Vec<usize> = vec![];

        for (index, _) in other.play_history.iter().enumerate() {
            if self.play_history.get(index).is_none() {
                new_entry_indices.push(index);
            }
        }

        if new_entry_indices.len() > 0 {
            let mut new_sounds = Vec::new();

            for new_entry_index in new_entry_indices {
                new_sounds.push(other.play_history.get(new_entry_index).unwrap().clone());
            }

            diff.new_sounds = Some(new_sounds)
        }


        diff

    }

    fn apply(&mut self, diff: &Self::Repr) {
        if let Some(new_sounds) = &diff.new_sounds {
            for new_sound in new_sounds {

                let sound = self.sound_cache.get(&new_sound.path).unwrap();

                play_sound(sound, PlaySoundParams::default());
            }
        }
    }

    fn identity() -> Self {
        SoundManager::new(Vec2::ZERO)
    }
}