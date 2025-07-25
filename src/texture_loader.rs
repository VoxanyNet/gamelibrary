use futures::executor::block_on;
use fxhash::FxHashMap;
use macroquad::texture::{self, load_texture, Texture2D};

use crate::log;

pub struct TextureLoader {
    pub cache: FxHashMap<String, Texture2D>
}

impl Default for TextureLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl TextureLoader {

    pub fn new() -> Self {
        TextureLoader { cache: FxHashMap::default() }
    }
    pub async fn get(&mut self, texture_path: &String) -> &Texture2D {
        // this can probably be optimized with a match statement but i cant figure it out the borrowing stuff
        if !self.cache.contains_key(texture_path) {

            let texture = load_texture(&texture_path).await.unwrap();
            
            texture.set_filter(texture::FilterMode::Nearest);

            self.cache.insert(texture_path.clone(), texture);

        }

        self.cache.get(texture_path).unwrap()
    }

    // pub fn get_blocking(&mut self, texture_path: &String) -> &Texture2D {
    //     // this can probably be optimized with a match statement but i cant figure it out the borrowing stuff
    //     if !self.cache.contains_key(texture_path) {

    //         let texture = block_on(
    //             load_texture(&texture_path)
    //         ).unwrap();
            
    //         texture.set_filter(texture::FilterMode::Nearest);

    //         self.cache.insert(texture_path.clone(), texture);

    //     }

    //     self.cache.get(texture_path).unwrap()
    // }
}