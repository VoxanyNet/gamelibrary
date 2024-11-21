use std::{fs, path::Path, time::{Duration, Instant}};


use macroquad::{color::WHITE, texture::{draw_texture, draw_texture_ex, DrawTextureParams}};
use serde::{Deserialize, Serialize};

use crate::texture_loader::TextureLoader;

#[derive(Serialize, Deserialize)]
struct AnimationMeta {
    frame_duration: f32
}

pub struct Animation {
    frames: Vec<String>,
    frame_duration: web_time::Duration,
    start_time: Option<web_time::Instant>,
    
}

impl Animation {

    pub fn new_from_directory(frames_directory: &String) -> Self {
        // need to handle error states!

        let mut paths: Vec<String> = vec![];

        let animation_meta: AnimationMeta = serde_json::from_str(&fs::read_to_string(format!("{}/animation_meta.json", frames_directory)).unwrap()).unwrap();

        let read_dir = fs::read_dir(frames_directory).unwrap();

        for path in read_dir {

            let is_file = path.as_ref().unwrap().file_type().unwrap().is_file();

            let is_png = path.as_ref().unwrap().path().to_str().unwrap().ends_with(".png");

            if is_file && is_png {
                paths.push(path.unwrap().path().to_str().unwrap().to_string()); // lol
            }
        };

        Self {
            frames: paths,
            frame_duration: Duration::from_secs_f32(animation_meta.frame_duration),
            start_time: None,
        }
    }

    pub fn start(&mut self) {
        self.start_time = Some(web_time::Instant::now());
    }

    pub fn stop(&mut self) {
        self.start_time = None;
    }

    pub fn current_frame(&self) -> usize {
        // if the user hasnt started the animation, just use now as the start time. it should just use the first frame
        let elapsed = self.start_time.unwrap_or(web_time::Instant::now()).elapsed(); 

        // determine the current frame based on the start time of the animation
        let current_frame = (elapsed.as_millis() / self.frame_duration.as_millis()) as usize % self.frames.len();

        return current_frame
    } 

    pub async fn draw(&mut self, x: f32, y: f32, textures: &mut TextureLoader, params: DrawTextureParams) {

        let current_frame = self.current_frame();

        let current_frame_texture = textures.get(
            &self.frames[current_frame]
        ).await;

        draw_texture_ex(current_frame_texture, x, y, WHITE, params);
    }
}