use std::collections::HashMap;



use diff::Diff;
use nalgebra::{point, vector};
use rapier2d::{dynamics::{CCDSolver, ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet, RigidBodySet}, geometry::{BroadPhase, ColliderSet, NarrowPhase}, pipeline::{PhysicsPipeline, QueryFilter, QueryPipeline}};
use serde::{Deserialize, Serialize};

use crate::{proxies::macroquad::math::vec2::Vec2, rigid_body::RigidBody};

pub type RigidBodyHandle = String;
pub type ColliderHandle = String;

#[derive(Serialize, Deserialize, Diff, PartialEq, Clone)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]
pub struct Space {
    rigid_bodies: HashMap<RigidBodyHandle, RigidBody>,
    gravity: f32
}

impl Space {

    pub fn new(gravity: f32) -> Self {
        Self {
            // we should probably seperate colliders and rigid bodies 
            rigid_bodies: HashMap::new(),
            gravity: gravity // we should probably move this elsewhere? i feel like this struct should only act as a wrapper for the rigid body set
        }
    }

    pub fn query_point(&mut self, point: Vec2) -> Vec<RigidBodyHandle> {
        // return a vector of rigid body handles that contains the point
        
        // convert proxy types to real types
        let (rigid_body_set, collider_set, proxy_map) = self.get_rigid_body_set();

        let mut query_pipeline = QueryPipeline::default();

        query_pipeline.update(&rigid_body_set, &collider_set);

        let point = point![point.x, point.y];
        let filter = QueryFilter::default();

        // vector containing all rigid bodies that contain the point
        let mut matching_bodies = vec![];


        query_pipeline.intersections_with_point(&rigid_body_set, &collider_set, &point, filter, |handle| {
            // Callback called on each collider with a shape containing the point.

            // search through all proxies to find the one with a matching
            // this is really badly optimized
            for (proxy_handle, (rigid_body_handle, collider_handle)) in proxy_map.clone() {
                if collider_handle == handle {
                    matching_bodies.push(proxy_handle);

                    break
                }
            }

            // Return `false` instead if we want to stop searching for other colliders containing this point.
            true
        }
        );

        matching_bodies

    }

    pub fn get_rigid_body_set(&mut self) -> (RigidBodySet, ColliderSet, HashMap<RigidBodyHandle, (rapier2d::dynamics::RigidBodyHandle, rapier2d::geometry::ColliderHandle)>) {
        // maps proxy handles to their real rigid bodies and colliders

        // this maps the rigid body proxy handles to the handles for their real rigid bodies and proxies, so the proxy types can be updated after they are stepped
        let mut rigid_body_map: HashMap<RigidBodyHandle, (rapier2d::dynamics::RigidBodyHandle, rapier2d::geometry::ColliderHandle)> = HashMap::new();

        let mut rigid_body_set = RigidBodySet::new();
        let mut collider_set = ColliderSet::new();

        for (rigid_body_proxy_handle, rigid_body_proxy) in self.rigid_bodies.iter_mut() {
            let rigid_body: rapier2d::dynamics::RigidBody = rigid_body_proxy.clone().into();
            let collider: rapier2d::geometry::Collider = rigid_body_proxy.collider.clone().into();

            let rigid_body_handle = rigid_body_set.insert(rigid_body);
            let collider_handle = collider_set.insert_with_parent(collider, rigid_body_handle, &mut rigid_body_set);

            rigid_body_map.insert(rigid_body_proxy_handle.clone(), (rigid_body_handle, collider_handle));
        }

        (
            rigid_body_set,
            collider_set,
            rigid_body_map
        )

    }      

    pub fn step(&mut self, owner: &String) {
        // convert all of the rigid bodies proxies to the actual rapier rigid body, step them all, then update the proxies using their real counterparts 
        

        // create all of the temporary structs needed to step the rigid bodies
        let gravity = vector![0., self.gravity];
        let integration_parameters = IntegrationParameters::default();
        let mut island_manager = IslandManager::default();
        let mut broad_phase = BroadPhase::new();
        let mut narrow_phase = NarrowPhase::new();
        let mut impulse_joint_set = ImpulseJointSet::new();
        let mut multibody_joint_set = MultibodyJointSet::new();
        let mut ccd_solver = CCDSolver::new();
        let mut query_pipeline = QueryPipeline::new();
        let physics_hooks = ();
        let event_handler = ();

        let mut physics_pipeline = PhysicsPipeline::new();

        // get the real rigid bodies and colliders from the proxies
        let (mut rigid_body_set, mut collider_set, rigid_body_map) = self.get_rigid_body_set();
    
        physics_pipeline.step(
            &gravity,
            &integration_parameters,
            &mut island_manager,
            &mut broad_phase,
            &mut narrow_phase,
            &mut rigid_body_set,
            &mut collider_set,
            &mut impulse_joint_set,
            &mut multibody_joint_set,
            &mut ccd_solver,
            Some(&mut query_pipeline),
            &physics_hooks,
            &event_handler
        );

        // update the proxies
        for (rigid_body_proxy_handle, (rigid_body_handle, collider_handle)) in rigid_body_map {
            
            let rigid_body_proxy = self.rigid_bodies.get_mut(&rigid_body_proxy_handle)
                .expect("Invalid rigid body proxy handle");

            // we only update the proxy rigid type if we own it
            if rigid_body_proxy.owner != *owner {
                continue;
            }

            // fetch the corresponding rigid body
            let rigid_body = rigid_body_set.get(rigid_body_handle)
                .expect("Invalid rigid body handle");

            // update the rigid body proxy with the actual rigid body
            rigid_body_proxy.update_from_rigid_body(rigid_body);

            let collider = collider_set.get(collider_handle)
                .expect("Invalid collider handle");

            rigid_body_proxy.collider.update_from_collider(collider);
        }



    }
    pub fn insert_rigid_body(&mut self, rigid_body: RigidBody) -> RigidBodyHandle {
        let handle: RigidBodyHandle = uuid::Uuid::new_v4().to_string();

        self.rigid_bodies.insert(handle.clone(), rigid_body);

        handle

    }

    pub fn get_rigid_body_mut(&mut self, rigid_body_handle: &RigidBodyHandle) -> Option<&mut RigidBody> {

        let rigid_body = self.rigid_bodies.get_mut(rigid_body_handle);

        rigid_body
    }

    pub fn get_rigid_body(&self, rigid_body_handle: &RigidBodyHandle) -> Option<&RigidBody> {
        let rigid_body = self.rigid_bodies.get(rigid_body_handle);

        rigid_body
    }
    
}