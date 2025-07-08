use std::{collections::HashMap, time::{Duration, Instant}};

// entity -> hashmap -> json -> diffed -> network -> json -> hashmap -> loaded
struct Timeline {
    frames: HashMap<Duration, String>,
    start: web_time::Instant
}

impl Timeline {
    
    fn reset(&mut self) {
        self.start = web_time::Instant::now();
    }

    fn get_current_frame(&self) {

    }
}