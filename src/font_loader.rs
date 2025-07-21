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
    pub async fn get(&mut self, font_path: &str) -> &Font {
        

        if !self.cache.contains_key(font_path) {

            let font = load_ttf_font(&font_path).await.unwrap();

            self.cache.insert(font_path.to_string(), font);

        }

        self.cache.get(font_path).unwrap()
    }    

    // preload a font into the cache
    pub async fn load(&mut self, font_path: &str) {
        if !self.cache.contains_key(font_path) {
            let font = load_ttf_font(&font_path).await.unwrap();

            self.cache.insert(font_path.to_string(), font);
        }
    }   

    // pub fn get_sync(&mut self, font_path: &String) -> &Font {
        

    //     if !self.cache.contains_key(font_path) {

    //         let font = block_on(load_ttf_font(&font_path)).unwrap();

    //         self.cache.insert(font_path.clone(), font);

    //     }

    //     self.cache.get(font_path).unwrap()
    // }   
}