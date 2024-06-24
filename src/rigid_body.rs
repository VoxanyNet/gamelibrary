use diff::Diff;
use rapier2d::{dynamics::RigidBodyBuilder, na::vector};
use serde::{Deserialize, Serialize};

use crate::{collider::Collider, proxies::macroquad::math::vec2::Vec2, space::ColliderHandle};

#[derive(Serialize, Deserialize, Diff, PartialEq, Clone)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]
pub enum RigidBodyType {
    Dynamic,
    Fixed,
    KinematicPositionBased,
    KinematicVelocityBased
}

impl From<rapier2d::dynamics::RigidBodyType> for RigidBodyType {
    fn from(value: rapier2d::dynamics::RigidBodyType) -> Self {
        match value {
            rapier2d::dynamics::RigidBodyType::Dynamic => RigidBodyType::Dynamic,
            rapier2d::dynamics::RigidBodyType::Fixed => RigidBodyType::Fixed,
            rapier2d::dynamics::RigidBodyType::KinematicPositionBased => RigidBodyType::KinematicPositionBased,
            rapier2d::dynamics::RigidBodyType::KinematicVelocityBased => RigidBodyType::KinematicVelocityBased
        }
    }
}

#[derive(Serialize, Deserialize, Diff, PartialEq, Clone)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]
pub struct RigidBody {
    pub position: Vec2,
    pub rotation: f32,
    pub angular_velocity: f32,
    pub velocity: Vec2,
    pub body_type: RigidBodyType,
    pub owner: String,
    pub collider: ColliderHandle
}

impl RigidBody {

    pub fn update_from_rigid_body(&mut self, value: &rapier2d::dynamics::RigidBody) {
        
        self.position = Vec2::new(value.position().translation.x, value.position().translation.y);
        self.velocity = Vec2::new(value.linvel().x, value.linvel().y);
        self.rotation = value.rotation().angle();
        self.angular_velocity = value.angvel();
        self.body_type = value.body_type().into();
    }

    pub fn as_rapier_rigid_body(&self) -> rapier2d::dynamics::RigidBody {
        match self.body_type {
            RigidBodyType::Dynamic => {
                RigidBodyBuilder::dynamic()
                    .rotation(self.rotation)
                    .angvel(self.angular_velocity)
                    .translation(vector![self.position.x, self.position.y])
                    .linvel(vector![self.velocity.x, self.velocity.y])
                    .build()
            },
            RigidBodyType::Fixed => {
                RigidBodyBuilder::fixed()
                    .rotation(self.rotation)
                    .angvel(self.angular_velocity)
                    .translation(vector![self.position.x, self.position.y])
                    .linvel(vector![self.velocity.x, self.velocity.y])
                    .build()
            },
            RigidBodyType::KinematicPositionBased => {
                RigidBodyBuilder::kinematic_position_based()
                    .rotation(self.rotation)
                    .angvel(self.angular_velocity)
                    .translation(vector![self.position.x, self.position.y])
                    .linvel(vector![self.velocity.x, self.velocity.y])
                    .build()
            },
            RigidBodyType::KinematicVelocityBased => {
                RigidBodyBuilder::kinematic_velocity_based()
                    .rotation(self.rotation)
                    .angvel(self.angular_velocity)
                    .translation(vector![self.position.x, self.position.y])
                    .linvel(vector![self.velocity.x, self.velocity.y])
                    .build()
            },
        }
    }
}