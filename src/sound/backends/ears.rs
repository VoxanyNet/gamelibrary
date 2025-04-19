use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::sound::soundmanager::{SoundHandle, SoundManager, SoundState};

#[cfg(feature = "3d-audio")]
use ears::{AudioController, SoundData};

#[cfg(feature = "3d-audio")]
/// Holds all sounds for client side
pub struct EarsSoundManager {
    sounds: HashMap<u64, ears::Sound>,
    // store sound data that corresponds to filename
    sound_data: HashMap<String, Rc<RefCell<SoundData>>>,
    listener_position: [f32; 3]
}

#[cfg(feature = "3d-audio")]
impl SoundManager for EarsSoundManager {

    fn new() -> Self {
        Self {
            sounds: HashMap::new(),
            sound_data: HashMap::new(),
            listener_position: [0., 0., 0.],
        }
    }

    fn update_listener_position(&mut self, new_listener_position: [f32; 3]) {
        self.listener_position = new_listener_position
    }

    fn sync_sound(&mut self, sound_handle: &mut SoundHandle) {
        // if the sound doesn't already exist on the client side we create it
        let client_sound = match self.sounds.get_mut(&sound_handle.id) {
            Some(client_sound) => {
                
                client_sound
            },
            None => {

                // load sound data from disk if we havent already
                let sound_data = match self.sound_data.get(&sound_handle.file_path) {
                    Some(sound_data) => sound_data.clone(),
                    None => {
                        let sound_data = SoundData::new(&sound_handle.file_path).unwrap();

                        self.sound_data.insert(sound_handle.file_path.clone(), Rc::new(RefCell::new(sound_data)));

                        self.sound_data.get(&sound_handle.file_path).unwrap().clone()


                    },
                };
                let client_sound = ears::Sound::new_with_data(sound_data).unwrap();

                self.sounds.insert(sound_handle.id, client_sound);
                self.sounds.get_mut(&sound_handle.id).unwrap()
            },
        };

        // the position this sound SHOULD be relative to the listener
        let new_position_relative_to_listener = [
            sound_handle.position[0] - self.listener_position[0],
            sound_handle.position[1] - self.listener_position[1],
            sound_handle.position[2] - self.listener_position[2]
        ];

        let current_position_relative_to_listener = client_sound.get_position();

        if current_position_relative_to_listener != new_position_relative_to_listener {
            client_sound.set_position(new_position_relative_to_listener);
        }

        // this is a situation where the sound sync goes the other way
        // the client tells the sound handle to update to stopped state, meaning that we have reached the end of playback
        if client_sound.get_state() == ears::State::Stopped {
            sound_handle.state = SoundState::Stopped;
        }

        // convert gamestate side sound state to client side sound state
        let sound_state: ears::State = sound_handle.state.clone().into();

        if sound_state != client_sound.get_state() {

            match sound_state {
                ears::State::Initial => {},
                ears::State::Playing => {
                    client_sound.play();
                },
                ears::State::Paused => {
                    client_sound.pause();
                }
                ears::State::Stopped => {
                    // we don't do anything if the handle is set to stopped
                    // this is because clients wont be perfectly synced and we don't want to cut off a client sound early
                    // the Stopped state means that the sound has FINISHED playing. we use Paused if we want to explicitly pause the sound on all clients
                }
            }
        }

        

    }
}
