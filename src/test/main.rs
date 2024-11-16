use std::time::Duration;

use gamelibrary::{animation_loader::AnimationLoader, texture_loader::TextureLoader};

use macroquad::prelude::*;



#[macroquad::main("Animation Test")]
async fn main() {

    let mut textures = TextureLoader::new();

    let mut animation_loader = AnimationLoader::new();

    let animation = animation_loader.get(&"example_animation".to_string());

    let mut draw_params = DrawTextureParams::default();

    draw_params.dest_size = Some(Vec2::new(96., 96.));

    animation.start();

    loop {
        clear_background(BLACK);

        animation.draw(100., 100., &mut textures, draw_params.clone()).await;

        next_frame().await
    }
}
