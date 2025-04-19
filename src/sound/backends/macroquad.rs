use std::collections::{HashMap, HashSet};

use macroquad::audio::{play_sound, PlaySoundParams, Sound};

use crate::sound::soundmanager::SoundManager;

pub struct MacroquadSoundManager {
    sound_data: HashMap<String, Sound>,
    listener_position: [f32; 3],
    sounds: HashSet<u64> // we cant actually update sounds but we can keep track of if we've played the sound yet
}

impl SoundManager for MacroquadSoundManager {
    fn new() -> Self where Self: Sized {
        Self {
            sound_data: HashMap::new(),
            listener_position: [0., 0., 0.],
            sounds: HashSet::new()
        }
    }

    fn update_listener_position(&mut self, new_listener_position: [f32; 3]) {
        self.listener_position = new_listener_position
    }

    fn sync_sound(&mut self, sound_handle: &mut crate::sound::soundmanager::SoundHandle) {

        // check if we've already played this sound
        if self.sounds.contains(&sound_handle.id) {
            return;
        };

        let sound = match self.sound_data.get(&sound_handle.file_path) {
            Some(sound) => sound,
            None => {
                let sound = futures::executor::block_on(
                    macroquad::audio::load_sound(&sound_handle.file_path)
                ).unwrap();

                self.sound_data.insert(sound_handle.file_path.clone(), sound);

                self.sound_data.get(&sound_handle.file_path).unwrap()
            },
        };

        let sound_parameters = PlaySoundParams {
            looped: false,
            volume: 1., // change this to fall off the further away the sound is 
        };

        play_sound(sound, sound_parameters);
        
    }
}

