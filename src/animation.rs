use std::fs;


use diff::Diff;
use macroquad::{color::WHITE, texture::{draw_texture_ex, DrawTextureParams}};
use serde::{Deserialize, Serialize};

use crate::{current_unix_millis, texture_loader::TextureLoader};

#[derive(Serialize, Deserialize, Diff, PartialEq, Clone)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]
pub struct TrackedFrames {
    pub frames: Frames,
    current_frame_index: usize
}

impl TrackedFrames {

    pub fn load_from_directory(frames_directory: &String) -> Self {
        Self {
            frames: Frames::load_from_directory(frames_directory),
            current_frame_index: 0,
        }
    }
    pub fn next_frame(&mut self) {
        self.current_frame_index += 1;

        if self.current_frame_index >= self.frames.paths.len() {
            self.current_frame_index = 0;
        }

    }

    pub fn set_frame(&mut self, new_frame_index: usize) {
        self.current_frame_index = new_frame_index;
    }

    pub fn current_frame(&self) -> &String {
        &self.frames.paths[self.current_frame_index]
    }
}


#[derive(Serialize, Deserialize, Diff, PartialEq, Clone)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]
pub struct Frames {
    paths: Vec<String>
}

impl Frames {

    pub fn len(&self) -> usize {
        self.paths.len()
    }
    
    pub fn load_from_directory(frames_directory: &String) -> Self {
        let mut paths: Vec<String> = vec![];

        let read_dir = fs::read_dir(frames_directory).unwrap();

        for dir_entry_result in read_dir {

            let dir_entry = dir_entry_result.as_ref().unwrap();

            let is_file = dir_entry.file_type().unwrap().is_file();

            let is_png = dir_entry.path().to_str().unwrap().ends_with(".png");

            let relative_path = dir_entry.path().to_str().unwrap().to_string();

            if is_file && is_png {
                paths.push(relative_path);
            }
        };

        // sort the vector of relative paths

        // YAYAYAYAY AI
        paths.sort_by_key(|path| {
            path.split('/')
                .last()                      // Get the file name
                .and_then(|filename| filename.split('.').next()) // Get the number part
                .and_then(|num_str| num_str.parse::<u32>().ok()) // Parse as a number
        });

        println!("{:?}", paths);

        Self {
            paths
        }

    }
}
#[derive(Serialize, Deserialize)]
struct AnimationMeta {
    frame_duration: u64 
}

#[derive(Serialize, Deserialize, Diff, PartialEq, Clone)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]
pub struct Animation {
    frames: Frames,
    frame_duration: u64,
    start_time: Option<u64>,
    pause_offset: Option<u64>, // the time at which we paused
}

impl Animation {

    pub fn new_from_directory(frames_directory: &String) -> Self {
        // need to handle error states!

        let animation_meta: AnimationMeta = serde_json::from_str(&fs::read_to_string(format!("{}/animation_meta.json", frames_directory)).unwrap()).unwrap();

        let frames = Frames::load_from_directory(frames_directory);

        Self {
            frames,
            frame_duration: animation_meta.frame_duration,
            start_time: None,
            pause_offset: None,
        }
    }

    /// Set the animation start point to now
    pub fn start(&mut self) {
        
        self.start_time = Some(current_unix_millis());
    }

    /// Delete the start time and pause offsets and stop the animation
    pub fn stop(&mut self) {
        self.start_time = None;
        self.pause_offset = None;

    }

    pub fn resume(&mut self) -> Result<(), ()>{
        let pause_offset = match &mut self.pause_offset {
            Some(pause_offset) => pause_offset,
            None => {
                return Result::Err(());
            },
        };

        let start_time = match &mut self.start_time {
            Some(start_time) => start_time,
            None => {
                return Result::Err(())
            },
        };

        // to make sure we start back at the tight frame we calculate where we were when we paused and use that as the new starting time
        *start_time = current_unix_millis() - *pause_offset;

        self.pause_offset = None;   

        return Result::Ok(());

        
    }

    /// Pause the animation
    pub fn pause(&mut self) -> Result<(), ()> {

        let start_time = match self.start_time {
            Some(start_time) => start_time,
            None => {return Result::Err(())},
        };

        self.pause_offset = Some(current_unix_millis() - start_time);

        return Result::Ok(());
    }

    pub fn current_frame(&self) -> usize {

        let start_time = match self.start_time {
            Some(start_time) => start_time,
            None => {
                // if we havent started the animation yet we just return the first frame
                return 0
            },
        };

        let elapsed = match self.pause_offset {
            Some(pause_offset) => {
                // if we are paused, we just return the elapsed time when we paused
                pause_offset
            },
            None => {
                // if we are currently playing, we return the actual elapsed time since we started the animation
                current_unix_millis() - start_time
            },
        };

        let current_frame = (elapsed / self.frame_duration) as usize % self.frames.paths.len();

        return current_frame
    } 

    pub async fn draw(&mut self, x: f32, y: f32, textures: &mut TextureLoader, params: DrawTextureParams) {

        let current_frame = self.current_frame();

        let current_frame_texture = textures.get(
            &self.frames.paths[current_frame]
        ).await;

        draw_texture_ex(current_frame_texture, x, y, WHITE, params);
    }
}