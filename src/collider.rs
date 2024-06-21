use diff::Diff;
use rapier2d::geometry::InteractionGroups;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Diff, PartialEq, Clone)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]

pub struct Collider {
    pub hx: f32,
    pub hy: f32,
    pub restitution: f32,
    pub mass: f32,
    pub owner: String,
    pub collision_groups: u32,
    pub collision_filter: u32
}

impl Collider {
    pub fn update_from_collider(&mut self, value: &rapier2d::geometry::Collider) {
        
        self.hx = value.shape().as_cuboid().unwrap().half_extents.x;
        self.hy = value.shape().as_cuboid().unwrap().half_extents.y;
        self.restitution = value.restitution();
        self.mass = value.mass();
        self.collision_groups = value.collision_groups().memberships.into();
        self.collision_filter = value.collision_groups().filter.into();
    }

    pub fn update_from_collider_mut(&mut self, value: &mut rapier2d::geometry::Collider) {
        
        self.hx = value.shape().as_cuboid().unwrap().half_extents.x;
        self.hy = value.shape().as_cuboid().unwrap().half_extents.y;
        self.restitution = value.restitution();
        self.mass = value.mass();
        self.collision_groups = value.collision_groups().memberships.into();
        self.collision_filter = value.collision_groups().filter.into();
    }
}


impl Into<rapier2d::geometry::Collider> for Collider {
    fn into(self) -> rapier2d::geometry::Collider {

        rapier2d::geometry::ColliderBuilder::cuboid(self.hx, self.hy)
            .restitution(self.restitution)
            .mass(self.mass)
            .collision_groups(InteractionGroups::new(self.collision_groups.into(), self.collision_filter.into()))
            .build()

    }
}

impl Into<rapier2d::geometry::Collider> for &Collider {
    fn into(self) -> rapier2d::geometry::Collider {
        
        rapier2d::geometry::ColliderBuilder::cuboid(self.hx, self.hy)
            .restitution(self.restitution)
            .mass(self.mass)
            .collision_groups(InteractionGroups::new(self.collision_groups.into(), self.collision_filter.into()))
            .build()

    }
}

impl Into<rapier2d::geometry::Collider> for &mut Collider {
    fn into(self) -> rapier2d::geometry::Collider {
        rapier2d::geometry::ColliderBuilder::cuboid(self.hx, self.hy)
            .restitution(self.restitution)
            .mass(self.mass)
            .collision_groups(InteractionGroups::new(self.collision_groups.into(), self.collision_filter.into()))
            .build()
    }
}