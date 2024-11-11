use macroquad::{camera::Camera2D, input::mouse_position, math::{Rect, Vec2}, window::screen_height};

pub mod timeline;
pub mod time;
pub mod space;
pub mod traits;
pub mod menu;
pub mod texture_loader;
pub mod sync;
pub mod animation;
pub mod animation_loader;

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