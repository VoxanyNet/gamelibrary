use macroquad::{miniquad::conf::Platform, window::Conf};


use gamelibrary::SoundManager;

fn window_conf() -> Conf {

    let mut platform = Platform::default();

    let mut conf = Conf {
        window_title: "Game".to_owned(),
        window_width: 1280,
        window_height: 720,
        window_resizable: false,
        platform: Platform::default(),
        fullscreen: false,
        ..Default::default()
    };
    //conf.platform.swap_interval = Some(0); // disable vsync

    conf
}

#[macroquad::main(window_conf)]
pub async fn main() {
    let mut sound_manager = SoundManager::new();

    sound_manager.load_sound("pistol.wav").await;

    
}