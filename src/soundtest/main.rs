use diff::Diff;
use gamelibrary::sound_manager::SoundManager;
use macroquad::{input::is_key_released, math::Vec2, miniquad::conf::Platform, window::Conf};

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
    let mut sound_manager = SoundManager::new(Vec2::ZERO);

    let before = sound_manager.clone();

    sound_manager.load_sound("pistol.wav").await;

    sound_manager.play_sound("pistol.wav", Vec2::ZERO);
    // sound_manager.play_sound("pistol.wav", Vec2::ZERO);
    // sound_manager.play_sound("pistol.wav", Vec2::ZERO);
    // sound_manager.play_sound("pistol.wav", Vec2::ZERO);

    sound_manager.play_sound("pistol.wav", Vec2::ZERO);
    // sound_manager.play_sound("pistol.wav", Vec2::ZERO);
    // sound_manager.play_sound("pistol.wav", Vec2::ZERO);
    // sound_manager.play_sound("pistol.wav", Vec2::ZERO);

    let diff = before.diff(&sound_manager);

    dbg!(diff);
    

    loop {

        if is_key_released(macroquad::input::KeyCode::Enter) {
            sound_manager.play_sound("pistol.wav", Vec2::ZERO);
        }
        macroquad::window::next_frame().await;
    }

    
}