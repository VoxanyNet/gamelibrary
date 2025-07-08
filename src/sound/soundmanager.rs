use diff::Diff;
use serde::{Deserialize, Serialize};

use crate::uuid_u32;

pub trait SoundManager {
    fn new() -> Self where Self: Sized;

    fn update_listener_position(&mut self, new_listener_position: [f32; 3]);

    async fn sync_sound(&mut self, sound_handle: &mut SoundHandle);
}

/// Synced structure for holding a sound's world position, volume, offset which will we sync with the client's client side sound
#[derive(Serialize, Deserialize, Diff, PartialEq, Clone)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]
pub struct SoundHandle {
    pub state: SoundState,
    pub position: [f32; 3],
    // id is used to match the sound handle with their client side counterpart
    pub id: u64,
    pub file_path: String

}

impl SoundHandle {
    pub fn new(file_path: &str, position: [f32; 3]) -> Self {
        Self {
            state: SoundState::Initial,
            position,
            id: uuid_u32() as u64,
            file_path: file_path.to_string(),
        }
    }

    pub fn play(&mut self) {
        self.state = SoundState::Playing
    }

    pub fn pause(&mut self) {
        self.state = SoundState::Paused
    }

}

#[derive(Serialize, Deserialize, Diff, PartialEq, Clone)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]
pub enum SoundState {
    Initial,
    Playing,
    Paused,
    Stopped
}

#[cfg(feature = "3d-audio")]
impl Into<ears::State> for SoundState {
    fn into(self) -> ears::State {
        match self {
            SoundState::Initial => ears::State::Initial,
            SoundState::Playing => ears::State::Playing,
            SoundState::Paused => ears::State::Paused,
            SoundState::Stopped => ears::State::Stopped,
        }
    }
}