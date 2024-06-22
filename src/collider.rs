use diff::Diff;
use rapier2d::na::vector;
use rapier2d::geometry::InteractionGroups;
use serde::{Deserialize, Serialize};

use crate::{proxies::macroquad::math::vec2::Vec2, space::RigidBodyHandle};

#[derive(Serialize, Deserialize, Diff, PartialEq, Clone)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]

pub struct Collider {
    pub position: Vec2,
    pub rotation: f32,
    pub hx: f32,
    pub hy: f32,
    pub restitution: f32,
    pub mass: f32,
    pub owner: String,
    pub collision_groups: u32,
    pub collision_filter: u32,
    pub parent: Option<RigidBodyHandle>
}

impl Collider {
    pub fn update_from_collider(&mut self, value: &rapier2d::geometry::Collider) {
        
        self.hx = value.shape().as_cuboid().unwrap().half_extents.x;
        self.hy = value.shape().as_cuboid().unwrap().half_extents.y;
        self.restitution = value.restitution();
        self.mass = value.mass();
        self.collision_groups = value.collision_groups().memberships.into();
        self.collision_filter = value.collision_groups().filter.into();
        self.position = Vec2::new(value.position().translation.x, value.position().translation.y);
        self.rotation = value.rotation().angle();
    }

    pub fn update_from_collider_mut(&mut self, value: &mut rapier2d::geometry::Collider) {
        
        self.hx = value.shape().as_cuboid().unwrap().half_extents.x;
        self.hy = value.shape().as_cuboid().unwrap().half_extents.y;
        self.restitution = value.restitution();
        self.mass = value.mass();
        self.collision_groups = value.collision_groups().memberships.into();
        self.collision_filter = value.collision_groups().filter.into();
        self.position = Vec2::new(value.position().translation.x, value.position().translation.y);
        self.rotation = value.rotation().angle();
    }
}


impl Into<rapier2d::geometry::Collider> for Collider {
    fn into(self) -> rapier2d::geometry::Collider {

        rapier2d::geometry::ColliderBuilder::cuboid(self.hx, self.hy)
            .restitution(self.restitution)
            .mass(self.mass)
            .collision_groups(InteractionGroups::new(self.collision_groups.into(), self.collision_filter.into()))
            .translation(vector![self.position.x, self.position.y])
            .rotation(self.rotation)
            .build()

    }
}

impl Into<rapier2d::geometry::Collider> for &Collider {
    fn into(self) -> rapier2d::geometry::Collider {
        
        rapier2d::geometry::ColliderBuilder::cuboid(self.hx, self.hy)
            .restitution(self.restitution)
            .mass(self.mass)
            .collision_groups(InteractionGroups::new(self.collision_groups.into(), self.collision_filter.into()))
            .translation(vector![self.position.x, self.position.y])
            .rotation(self.rotation)
            .build()

    }
}

impl Into<rapier2d::geometry::Collider> for &mut Collider {
    fn into(self) -> rapier2d::geometry::Collider {
        rapier2d::geometry::ColliderBuilder::cuboid(self.hx, self.hy)
            .restitution(self.restitution)
            .mass(self.mass)
            .collision_groups(InteractionGroups::new(self.collision_groups.into(), self.collision_filter.into()))
            .translation(vector![self.position.x, self.position.y])
            .rotation(self.rotation)
            .build()
    }
}