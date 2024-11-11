use fxhash::FxHashMap;

use crate::animation::Animation;

pub struct AnimationLoader {
    pub cache: FxHashMap<String, Animation>
}

impl AnimationLoader {
    pub fn new() -> Self {
        AnimationLoader {
            cache: FxHashMap::default(),
        }
    }

    pub fn get(&mut self, animation_path: &String) -> &Animation {
        
        if !self.cache.contains_key(animation_path) {
            let animation = Animation::new_from_directory(animation_path);

            self.cache.insert(animation_path.clone(), animation);
        };

        self.cache.get(animation_path).unwrap()

    }
}