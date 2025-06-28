use futures::executor::block_on;
use fxhash::FxHashMap;
use macroquad::text::{load_ttf_font, Font};

pub struct FontLoader {
    pub cache: FxHashMap<String, Font>
}

impl FontLoader {

    pub fn new() -> Self {
        Self {
            cache: FxHashMap::default(),
        }
    }
    pub fn get(&mut self, font_path: &String) -> &Font {
        

        if !self.cache.contains_key(font_path) {

            let font = block_on(load_ttf_font(&font_path)).unwrap();

            self.cache.insert(font_path.clone(), font);

        }

        self.cache.get(font_path).unwrap()
    }    
}