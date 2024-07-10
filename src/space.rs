use std::collections::HashMap;



use diff::Diff;
use macroquad::math::Vec2;
use nalgebra::{point, vector};
use rapier2d::{crossbeam, dynamics::{rigid_body, CCDSolver, ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet, RigidBodyHandle, RigidBodySet}, geometry::{BroadPhase, ColliderHandle, ColliderSet, DefaultBroadPhase, NarrowPhase}, pipeline::{ChannelEventCollector, PhysicsPipeline, QueryFilter, QueryPipeline}};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Space {
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub gravity: nalgebra::Matrix<f32, nalgebra::Const<2>, nalgebra::Const<1>, nalgebra::ArrayStorage<f32, 2, 1>>,
    pub integration_parameters: IntegrationParameters,
    #[serde(skip)]
    pub physics_pipeline: PhysicsPipeline,
    pub island_manager: IslandManager,
    pub broad_phase: DefaultBroadPhase,
    pub narrow_phase: NarrowPhase,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
    pub query_pipeline: QueryPipeline,
    pub physics_hooks: (),
    pub event_handler: (),
}

impl Space {

    pub fn new() -> Self {
        let rigid_body_set = RigidBodySet::new();
        let collider_set = ColliderSet::new();
    
        /* Create other structures necessary for the simulation. */
        let gravity = vector![0.0, -9.81];
        let integration_parameters = IntegrationParameters::default();
        let physics_pipeline = PhysicsPipeline::new();
        let island_manager = IslandManager::new();
        let broad_phase = DefaultBroadPhase::new();
        let narrow_phase = NarrowPhase::new();
        let impulse_joint_set = ImpulseJointSet::new();
        let multibody_joint_set = MultibodyJointSet::new();
        let ccd_solver = CCDSolver::new();
        let query_pipeline = QueryPipeline::new();
        let physics_hooks = ();
        let event_handler = ();

        Self { 
            rigid_body_set, 
            collider_set, 
            gravity, 
            integration_parameters, 
            physics_pipeline, 
            island_manager, 
            broad_phase, 
            narrow_phase, 
            impulse_joint_set, 
            multibody_joint_set, 
            ccd_solver, 
            query_pipeline, 
            physics_hooks, 
            event_handler 
        }
    }

    pub fn step(&mut self, owned_rigid_bodies: Vec<RigidBodyHandle>, owned_colliders: Vec<ColliderHandle>) {
        // convert all of the rigid bodies proxies to the actual rapier rigid body, step them all, then update the proxies using their real counterparts 
    
        // let (collision_send, collision_recv) = crossbeam::channel::unbounded();
        // let (contact_force_send, contact_force_recv) = crossbeam::channel::unbounded();
        // let event_handler = ChannelEventCollector::new(collision_send, contact_force_send);

        // any colliders/bodies we do not own we will return to their original state here
        let rigid_body_set_before = self.rigid_body_set.clone();
        let collider_set_before = self.collider_set.clone();
    
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &self.physics_hooks,
            &self.event_handler
        );

        for (rigid_body_handle, rigid_body) in self.rigid_body_set.iter_mut() {
            if owned_rigid_bodies.contains(&rigid_body_handle) {
                continue;
            }

            // figure out how to make this not clone
            *rigid_body = rigid_body_set_before.get(rigid_body_handle).expect("Unable to find old version of rigid body before it was updated").clone();
        }

        for (collider_handle, collider) in self.collider_set.iter_mut() {
            if owned_colliders.contains(&collider_handle) {
                continue;
            }

            *collider = collider_set_before.get(collider_handle).expect("Unable to find old version of collider before it was updated").clone();
        }

        // update events
        // while let Ok(collision_event) = collision_recv.try_recv() {
        //     // Handle the collision event.
        //     println!("Received collision event: {:?}", collision_event);
        // }

    }
    
}

#[derive(Serialize, Deserialize)]
pub struct SpaceDiff {
    // for some reason i cant use RigidBodySetDiff directly
    rigid_body_set: Option<<RigidBodySet as Diff>::Repr>,
    collider_set: Option<<ColliderSet as Diff>::Repr>,
    gravity: Option<nalgebra::Matrix<f32, nalgebra::Const<2>, nalgebra::Const<1>, nalgebra::ArrayStorage<f32, 2, 1>>>,
    // might wanna add the rest of the fields
}

impl Diff for Space {
    type Repr = SpaceDiff; 

    fn diff(&self, other: &Self) -> Self::Repr {
        let mut diff = SpaceDiff {
            rigid_body_set: None,
            collider_set: None,
            gravity: None,
        };

        if other.rigid_body_set != self.rigid_body_set {
            diff.rigid_body_set = Some(self.rigid_body_set.diff(&other.rigid_body_set))
        }

        if other.collider_set != self.collider_set {
            diff.collider_set = Some(self.collider_set.diff(&other.collider_set))
        }

        if other.gravity != self.gravity {
            diff.gravity = Some(other.gravity)
        }

        diff

    }

    fn apply(&mut self, diff: &Self::Repr) {
        if let Some(rigid_body_set_diff) = &diff.rigid_body_set {
            self.rigid_body_set.apply(rigid_body_set_diff);
        }

        if let Some(collider_set_diff) = &diff.collider_set {
            self.collider_set.apply(collider_set_diff);
        }

        if let Some(gravity) = &diff.gravity {
            self.gravity = *gravity;
        }
    }

    fn identity() -> Self {
        Space::new()
    }
}