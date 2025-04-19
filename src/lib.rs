
use core::fmt;
use std::{cell::RefCell, collections::HashMap, rc::Rc, time::{SystemTime, UNIX_EPOCH}};

use diff::Diff;
use fxhash::FxHashMap;
use macroquad::{camera::Camera2D, color::Color, input::mouse_position, math::{vec2, Rect, Vec2, Vec3}, texture::{draw_texture_ex, DrawTextureParams, Texture2D}, window::screen_height};
use rapier2d::prelude::ColliderHandle;
use serde::{de::{self, MapAccess, Visitor}, ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};
use space::Space;

pub mod timeline;
pub mod time;
pub mod space;
pub mod traits;
pub mod menu;
pub mod texture_loader;
pub mod sync;
pub mod animation;
pub mod animation_loader;
pub mod swapiter;
pub mod arenaiter;
pub mod sound;

pub fn get_angle_to_mouse(point: Vec2, camera_rect: &Rect) -> f32 {

    let mouse_pos = rapier_mouse_world_pos(camera_rect);

    let distance_to_mouse = Vec2::new(
        mouse_pos.x - point.x,
        mouse_pos.y - point.y 
    );

    distance_to_mouse.x.atan2(distance_to_mouse.y)
}

/// Get the relative top left of a collider
pub fn collider_top_left_pos(space: &Space, collider_handle: ColliderHandle) -> Vec2 {
    let collider = space.collider_set.get(collider_handle).unwrap();

    let shape = collider.shape().as_cuboid().unwrap();

    Vec2::new(-shape.half_extents.x, -shape.half_extents.y)
    
}
pub fn rotate_point(point: Vec2, center: Vec2, theta: f32) -> Vec2 {

    // translate the point to the origin
    let translated_x = point.x - center.x;
    let translated_y = point.y - center.y;

    // apply the rotation matrix
    let rotated_x = translated_x * theta.cos() - translated_y * theta.sin();
    let rotated_y = translated_x * theta.sin() + translated_y * theta.cos();

    // translate back to the original position
    Vec2::new(rotated_x + center.x, rotated_y + center.y)
}
pub fn draw_texture_rapier(
    texture: &Texture2D, 
    x: f32, 
    y: f32, 
    color: Color, 
    params: DrawTextureParams
) {

    let draw_pos = rapier_to_macroquad(
        &vec2(x, y)
    );

    draw_texture_ex(
        texture, 
        draw_pos.x, 
        draw_pos.y, 
        color,
        params
    );
}
pub fn current_unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}

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

#[cfg(target_arch = "x86_64")]
pub fn log(message: &str) {
    println!("{message}");
}

#[cfg(target_arch = "wasm32")]
pub fn log(message: &str) {
    web_sys::console::log_1(&message.into());
}
pub fn uuid() -> String {

    // WTF
    let mut buf = [0u8; 4];
    getrandom::getrandom(&mut buf).unwrap();
    u32::from_be_bytes(buf).to_string()

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