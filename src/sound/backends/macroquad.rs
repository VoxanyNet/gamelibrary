use std::collections::{HashMap, HashSet};

use macroquad::audio::{play_sound, PlaySoundParams, Sound};

use crate::sound::soundmanager::SoundManager;

pub struct MacroquadSoundManager {
    sound_data: HashMap<String, Sound>,
    listener_position: [f32; 3],
    sounds: HashSet<u64>, // we cant actually update sounds but we can keep track of if we've played the sound yet
    stupid_connection_fix: bool
}

impl SoundManager for MacroquadSoundManager {
    fn new() -> Self where Self: Sized {
        Self {
            sound_data: HashMap::new(),
            listener_position: [0., 0., 0.],
            sounds: HashSet::new(),
            stupid_connection_fix: false
        }
    }

    fn set_stupid_connection_fix(&mut self, toggle: bool) {
        self.stupid_connection_fix = toggle;
    }

    fn update_listener_position(&mut self, new_listener_position: [f32; 3]) {
        self.listener_position = new_listener_position
    }

    async fn sync_sound(&mut self, sound_handle: &mut crate::sound::soundmanager::SoundHandle) {

        // only play the sound if the state is Playing
        match sound_handle.state {
            crate::sound::soundmanager::SoundState::Playing => {},
            _ => return
        }
        // check if we've already played this sound
        if self.sounds.contains(&sound_handle.id) {
            return;
        };

        self.sounds.insert(sound_handle.id);

        let sound = match self.sound_data.get(&sound_handle.file_path) {
            Some(sound) => sound,
            None => {
                let sound = macroquad::audio::load_sound(&sound_handle.file_path).await.unwrap();

                self.sound_data.insert(sound_handle.file_path.clone(), sound);

                self.sound_data.get(&sound_handle.file_path).unwrap()
            },
        };

        let sound_parameters = PlaySoundParams {
            looped: false,
            volume: 1., // change this to fall off the further away the sound is 
        };

        // this is stupid
        // i cant find a way to track if a sound is done playing with macroquad audio. so all the audio tha thas been played during the game will be played all at once when connecting. we set this value on the first frame to ignore any sounds played on the first frame
        if self.stupid_connection_fix {
            return ;
        }

        play_sound(sound, sound_parameters);
        
    }
}

