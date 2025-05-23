use std::{collections::HashMap, time::{Duration, Instant}};
use macroquad::math::Vec2;
use rapier2d::prelude::{ColliderHandle, RigidBodyHandle};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Serialize, Deserialize, diff::Diff, PartialEq, Clone)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]
pub struct PhysicsObject {
    pos: Vec2,
    velocity: Vec2,
    size: Vec2
}

#[derive(Debug, Serialize, Deserialize, diff::Diff, PartialEq, Clone, Hash, Eq)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]
pub struct PhysicsObjectHandle {
    id: u64
}

#[derive(Debug, Serialize, Deserialize, diff::Diff, PartialEq, Clone)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]
pub struct Space {
    
   pub physics_objects: std::collections::HashMap<PhysicsObjectHandle, PhysicsObject>,

   rapier_handle_map: std::co
    
}




impl Space {

    pub fn new() -> Self {
        Self {
            physics_objects: HashMap::new(),
            rapier_handle_map: HashMap::new()
        }
    }

    pub fn step(&mut self, owned_physics_objects: Vec<PhysicsObjectHandle>, dt: &Instant) {

        for

    }
    
}