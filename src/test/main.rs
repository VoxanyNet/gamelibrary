
use gamelibrary::{animation_loader::AnimationLoader, menu::Menu, texture_loader::TextureLoader};

use macroquad::prelude::*;



#[macroquad::main("Animation Test")]
async fn main() {

    let mut menu = Menu::new(Vec2::new(0., 20.), GRAY, "assets/fonts/CutePixel.ttf".to_string(), None, None);

    menu.add_button("Stop".to_string());
    menu.add_button("Play".to_string());
    menu.add_button("Pause".to_string());
    menu.add_button("Resume".to_string());
    

    let mut textures = TextureLoader::new();

    let mut animation_loader = AnimationLoader::new();

    let animation = animation_loader.get(&"example_animation".to_string());

    let mut draw_params = DrawTextureParams::default();

    draw_params.dest_size = Some(Vec2::new(96., 96.));

    animation.start();


    loop {

        menu.draw().await;

        menu.update(None);

        for item in menu.clone().get_menu_items() {

            // i still cannot figure out why this is required but otherwise it gets stuck in an infinite loop in the for loop
            if !item.clicked {
                continue;
            }

            match item.text.as_str() {
                "Stop" => {
                    animation.stop();

                    
                },
                "Play" => {
                    animation.start();

                }

                "Pause" => {
                    animation.pause().unwrap();
                       
                }

                "Resume" => {
                    animation.resume().unwrap();
                       
                }
                _ => {}
            }
        }

        animation.draw(100., 100., &mut textures, draw_params.clone()).await;

        next_frame().await
    }
}
