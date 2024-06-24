use std::collections::HashMap;



use diff::Diff;
use nalgebra::{point, vector};
use rapier2d::{crossbeam, dynamics::{CCDSolver, ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet, RigidBodySet}, geometry::{BroadPhase, ColliderSet, NarrowPhase}, pipeline::{ChannelEventCollector, PhysicsPipeline, QueryFilter, QueryPipeline}};
use serde::{Deserialize, Serialize};

use crate::{collider::Collider, proxies::macroquad::math::vec2::Vec2, rigid_body::RigidBody};

#[derive(Serialize, Deserialize, Diff, PartialEq, Clone, Eq, PartialOrd, Ord, Hash)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]
pub struct RigidBodyHandle {
    key: String
}

#[derive(Serialize, Deserialize, Diff, PartialEq, Clone, Eq, PartialOrd, Ord, Hash)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]
pub struct ColliderHandle {
    key: String
}

#[derive(Serialize, Deserialize, Diff, PartialEq, Clone)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]
pub struct Space {
    rigid_bodies: HashMap<RigidBodyHandle, RigidBody>,
    colliders: HashMap<ColliderHandle, Collider>,
    gravity: f32
}

impl Space {

    pub fn new(gravity: f32) -> Self {
        Self {
            // we should probably seperate colliders and rigid bodies 
            rigid_bodies: HashMap::new(),
            colliders: HashMap::new(),
            gravity: gravity // we should probably move this elsewhere? i feel like this struct should only act as a wrapper for the rigid body set
        }
    }

    pub fn query_point(&mut self, point: Vec2) -> Vec<ColliderHandle> {
        // return a vector of collider handles that contains the point
        
        // convert proxy types to real types
        let (mut rigid_body_set, rigid_body_proxy_map) = self.get_rigid_body_set();
        let (collider_set, collider_proxy_map) = self.get_collider_set(&mut rigid_body_set, &rigid_body_proxy_map);

        let mut query_pipeline = QueryPipeline::default();

        query_pipeline.update(&rigid_body_set, &collider_set);

        let point = point![point.x, point.y];
        let filter = QueryFilter::default();

        // vector containing all colliders that contain the point
        let mut matching_colliders = vec![];
        

        query_pipeline.intersections_with_point(&rigid_body_set, &collider_set, &point, filter, |handle| {
            // Callback called on each collider with a shape containing the point.

            // search through all proxies to find the one with a matching
            // this is really badly optimized
            for (proxy_handle, collider_handle) in collider_proxy_map.clone() {
                if collider_handle == handle {
                    matching_colliders.push(proxy_handle);

                    break
                }
            }

            // Return `false` instead if we want to stop searching for other colliders containing this point.
            true
        }
        );

        matching_colliders

    }

    pub fn get_rigid_body_set(&mut self) -> (RigidBodySet, HashMap<RigidBodyHandle, rapier2d::dynamics::RigidBodyHandle>) {
        // maps proxy handles to their real rigid bodies

        // this maps the rigid body proxy handles to the handles for their real rigid bodies and proxies, so the proxy types can be updated after they are stepped
        let mut rigid_body_map: HashMap<RigidBodyHandle, rapier2d::dynamics::RigidBodyHandle> = HashMap::new();

        let mut rigid_body_set = RigidBodySet::new();

        for (rigid_body_proxy_handle, rigid_body_proxy) in self.rigid_bodies.iter_mut() {
            let rigid_body: rapier2d::dynamics::RigidBody = rigid_body_proxy.as_rapier_rigid_body();

            let rigid_body_handle = rigid_body_set.insert(rigid_body);

            rigid_body_map.insert(rigid_body_proxy_handle.clone(), rigid_body_handle);
        }

        (
            rigid_body_set,
            rigid_body_map
        )

    }

    pub fn get_collider_set(&mut self, rigid_body_set: &mut RigidBodySet, rigid_body_map: &HashMap<RigidBodyHandle, rapier2d::dynamics::RigidBodyHandle>) -> (ColliderSet, HashMap<ColliderHandle, rapier2d::geometry::ColliderHandle>) {
        // maps proxy handles to their real colliders

        // this maps the collider proxy handles to the handles for their real collider bodies, so the proxy types can be updated after they are stepped
        let mut collider_map: HashMap<ColliderHandle, rapier2d::geometry::ColliderHandle> = HashMap::new();

        let mut collider_set = ColliderSet::new();

        for (collider_proxy_handle, collider_proxy) in self.colliders.iter_mut() {
            let real_collider: rapier2d::geometry::Collider = collider_proxy.as_rapier_collider();

            let collider_handle = match &collider_proxy.parent {
                Some(proxy_parent_rigid_body_handle) => {

                    let real_parent_rigid_body_handle = rigid_body_map.get(&proxy_parent_rigid_body_handle).unwrap();

                    collider_set.insert_with_parent(real_collider, *real_parent_rigid_body_handle, rigid_body_set)
                },
                None => collider_set.insert(real_collider)
            };

            collider_map.insert(collider_proxy_handle.clone(), collider_handle);
        }

        (
            collider_set,
            collider_map
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

        let (collision_send, collision_recv) = crossbeam::channel::unbounded();
        let (contact_force_send, contact_force_recv) = crossbeam::channel::unbounded();
        let event_handler = ChannelEventCollector::new(collision_send, contact_force_send);

        let physics_hooks = ();

        let mut physics_pipeline = PhysicsPipeline::new();

        // get the real rigid bodies and colliders from the proxies
        let (mut rigid_body_set, rigid_body_map) = self.get_rigid_body_set();
        let (mut collider_set, collider_map) = self.get_collider_set(&mut rigid_body_set, &rigid_body_map);
    
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

        // update events
        while let Ok(collision_event) = collision_recv.try_recv() {
            // Handle the collision event.
            println!("Received collision event: {:?}", collision_event);
        }

        // update the rigid body proxies
        for (rigid_body_proxy_handle, rigid_body_handle) in rigid_body_map {
            
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
        }

        // update the collider proxies
        for (collider_proxy_handle, collider_handle) in collider_map {
            
            let collider_proxy = self.colliders.get_mut(&collider_proxy_handle)
                .expect("Invalid collider proxy handle");

            // we only update the proxy rigid type if we own it
            if collider_proxy.owner != *owner {
                continue;
            }

            // fetch the corresponding rigid body
            let collider = collider_set.get(collider_handle)
                .expect("Invalid rigid body handle");

            // update the rigid body proxy with the actual rigid body
            collider_proxy.update_from_collider(collider);
        }



    }

    pub fn insert_collider(&mut self, collider: Collider) -> ColliderHandle {
        let handle = ColliderHandle{
            key: uuid::Uuid::new_v4().to_string()
        };


        self.colliders.insert(handle.clone(), collider);

        handle
    }

    pub fn insert_rigid_body(&mut self, rigid_body: RigidBody) -> RigidBodyHandle {

        let handle = RigidBodyHandle {
            key: uuid::Uuid::new_v4().to_string()
        };

        if !self.colliders.contains_key(&rigid_body.collider) {
            panic!("specified collider does not exist")
        }

        // update the collider attached to the rigid body
        for (collider_handle, collider) in &mut self.colliders {
            if rigid_body.collider == *collider_handle {
                collider.parent = Some(handle.clone());
                
                break;
            }
        }

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

    pub fn get_collider_mut(&mut self, collider_handle: &ColliderHandle) -> Option<&mut Collider> {
        let collider = self.colliders.get_mut(collider_handle);

        collider
    }

    pub fn get_collider(&self, collider_handle: &ColliderHandle) -> Option<&Collider> {
        let collider = self.colliders.get(collider_handle);

        collider
    }
    
}