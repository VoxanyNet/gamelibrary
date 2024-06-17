use std::time::{SystemTime, UNIX_EPOCH};

use macroquad::{math::Vec2, window::screen_height};

pub mod timeline;
pub mod proxies;
pub mod time;
pub mod rigid_body;
pub mod collider;
pub mod space;
pub mod traits;
pub mod menu;

pub fn uuid() -> String {
    // AHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHH
    macroquad::rand::srand(SystemTime::now().duration_since(UNIX_EPOCH).expect("we went back in time!").as_nanos() as u64);
    macroquad::rand::rand().to_string()
}

pub fn translate_coordinates(macroquad_coords: &Vec2) -> Vec2 {
    // translate macroquad coords to rapier coords
    Vec2 { 
        x: macroquad_coords.x, 
        y: (macroquad_coords.y * -1.) + screen_height()
    }
}