use std::{fs, path, time::{Duration, Instant}};


use macroquad::{color::WHITE, texture::draw_texture};

use crate::texture_loader::TextureLoader;

pub struct Animation {
    frames: Vec<String>,
    frame_duration: Duration,
    current_frame: usize,
    last_frame: Instant
}

impl Animation {

    pub fn new_from_directory(frames_directory: &String, frame_duration: Duration) -> Self {
        // need to handle error states!

        let mut paths: Vec<String> = vec![];

        let read_dir = fs::read_dir(frames_directory).unwrap();

        for path in read_dir {

            let is_file = path.as_ref().unwrap().file_type().unwrap().is_file();

            if is_file {
                paths.push(path.unwrap().path().to_str().unwrap().to_string()); // lol
            }
        };

        Self {
            frames: paths,
            frame_duration,
            current_frame: 0,
            last_frame: Instant::now(),
        }
    }
    pub async fn draw(&mut self, x: f32, y: f32, textures: &mut TextureLoader) {

        if self.last_frame.elapsed() >= self.frame_duration {
            self.current_frame += 1
        }

        let current_frame_texture = textures.get(
            &self.frames[self.current_frame]
        ).await;

        draw_texture(current_frame_texture, x, y, WHITE);
    }
}