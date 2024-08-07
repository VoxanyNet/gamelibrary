use std::time::{SystemTime, UNIX_EPOCH};

use macroquad::{camera::{Camera2D}, input::mouse_position, math::{Rect, Vec2}, window::screen_height};

pub mod timeline;
pub mod time;
pub mod space;
pub mod traits;
pub mod menu;
pub mod texture_loader;
pub mod sync;

pub fn mouse_world_pos(camera_rect: &Rect) -> Vec2 {
    let mouse_pos = mouse_position();

    let mut camera = Camera2D::from_display_rect(*camera_rect);
    camera.zoom.y = -camera.zoom.y;

    camera.screen_to_world(mouse_pos.into())

}

pub fn rapier_mouse_world_pos(camera_rect: &Rect) -> Vec2 {
    macroquad_to_rapier(
        &mouse_world_pos(camera_rect)
    )
}

pub fn uuid() -> String {

    macroquad::rand::srand(SystemTime::now().duration_since(UNIX_EPOCH).expect("we went back in time!").as_nanos() as u64);
    macroquad::rand::rand().to_string()
}

pub fn macroquad_to_rapier(macroquad_coords: &Vec2) -> Vec2 {

    // translate macroquad coords to rapier coords
    Vec2 { 
        x: macroquad_coords.x, 
        y: (macroquad_coords.y * -1.) + screen_height()
    }
}

pub fn rapier_to_macroquad(rapier_coords: &Vec2) -> Vec2 {
    Vec2 {
        x: rapier_coords.x,
        y: (rapier_coords.y * -1.) + screen_height()
    }
}