use std::time::{Duration, Instant};

use diff::Diff;
use nalgebra::vector;
use rapier2d::{crossbeam::{self, channel::Receiver}, dynamics::{CCDSolver, ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet, RigidBodyHandle, RigidBodySet}, geometry::{ColliderHandle, ColliderSet, DefaultBroadPhase, NarrowPhase}, pipeline::{PhysicsPipeline, QueryPipeline}, prelude::{ChannelEventCollector, CollisionEvent}};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Serialize)]
pub struct Space {
    
    pub rigid_body_set: RigidBodySet,
    #[serde(skip)]
    pub collision_recv: Receiver<CollisionEvent>,
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
    #[serde(skip)]
    pub event_handler: ChannelEventCollector,
    #[serde(skip)]
    pub last_step: Instant
}

impl<'de> Deserialize<'de> for Space {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct SpaceHelper {
            rigid_body_set: RigidBodySet,
            collider_set: ColliderSet,
            gravity: nalgebra::Matrix<f32, nalgebra::Const<2>, nalgebra::Const<1>, nalgebra::ArrayStorage<f32, 2, 1>>,
            integration_parameters: IntegrationParameters,
            island_manager: IslandManager,
            broad_phase: DefaultBroadPhase,
            narrow_phase: NarrowPhase,
            impulse_joint_set: ImpulseJointSet,
            multibody_joint_set: MultibodyJointSet,
            ccd_solver: CCDSolver,
            query_pipeline: QueryPipeline,
        }

        let helper = SpaceHelper::deserialize(deserializer)?;

        let (collision_send, collision_recv) = crossbeam::channel::unbounded();
        let (contact_force_send, _contact_force_recv) = crossbeam::channel::unbounded();
        let event_handler = ChannelEventCollector::new(collision_send, contact_force_send);

        Ok(Space {
            rigid_body_set: helper.rigid_body_set,
            collision_recv,
            collider_set: helper.collider_set,
            gravity: helper.gravity,
            integration_parameters: helper.integration_parameters,
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: helper.island_manager,
            broad_phase: helper.broad_phase,
            narrow_phase: helper.narrow_phase,
            impulse_joint_set: helper.impulse_joint_set,
            multibody_joint_set: helper.multibody_joint_set,
            ccd_solver: helper.ccd_solver,
            query_pipeline: helper.query_pipeline,
            event_handler,
            physics_hooks: (),
            last_step: Instant::now()
        })
    }
}

impl Clone for Space {
    fn clone(&self) -> Self {

        let (collision_send, collision_recv) = crossbeam::channel::unbounded();
        let (contact_force_send, _contact_force_recv) = crossbeam::channel::unbounded();
        let event_handler = ChannelEventCollector::new(collision_send, contact_force_send);

        Self {
            rigid_body_set: self.rigid_body_set.clone(),
            collider_set: self.collider_set.clone(),
            gravity: self.gravity.clone(),
            integration_parameters: self.integration_parameters.clone(),
            physics_pipeline: self.physics_pipeline.clone(),
            island_manager: self.island_manager.clone(),
            broad_phase: self.broad_phase.clone(),
            narrow_phase: self.narrow_phase.clone(),
            impulse_joint_set: self.impulse_joint_set.clone(),
            multibody_joint_set: self.multibody_joint_set.clone(),
            ccd_solver: self.ccd_solver.clone(),
            query_pipeline: self.query_pipeline.clone(),
            physics_hooks: self.physics_hooks.clone(),
            event_handler,
            collision_recv,
            last_step: web_time::Instant::now()
        }
    }
}

impl PartialEq for Space {
    fn eq(&self, other: &Self) -> bool {
        other.rigid_body_set == self.rigid_body_set && other.collider_set == self.collider_set
    }

    fn ne(&self, other: &Self) -> bool {
        other.rigid_body_set != self.rigid_body_set || other.collider_set != self.collider_set
    }
}

impl Space {

    pub fn new() -> Self {
        let rigid_body_set = RigidBodySet::new();
        let collider_set = ColliderSet::new();


        let (collision_send, collision_recv) = crossbeam::channel::unbounded();
        let (contact_force_send, _contact_force_recv) = crossbeam::channel::unbounded();
        let event_handler = ChannelEventCollector::new(collision_send, contact_force_send);
    
        /* Create other structures necessary for the simulation. */
        let gravity = vector![0.0, 0.];
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
        let last_step = Instant::now();

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
            event_handler,
            collision_recv,
            last_step
        }
    }

    pub fn step(&mut self, owned_rigid_bodies: &Vec<RigidBodyHandle>, owned_colliders: &Vec<ColliderHandle>, dt: &web_time::Instant) {


        // only step space every 1/60th of a second
        if self.last_step.elapsed().as_micros() < 8330 {
            return;
        }

        self.last_step = Instant::now();

        self.integration_parameters.dt = 1./120.;

        //self.last_step = web_time::Instant::now();
        // any colliders/bodies we do not own we will return to their original state here
        let rigid_body_set_before = self.rigid_body_set.clone();
        let collider_set_before = self.collider_set.clone();
        

        for (rigid_body_handle, rigid_body) in self.rigid_body_set.iter_mut() {

            // this is a temporary workaround but i think we are failing to sync sleep states
            rigid_body.wake_up(true);   
            if owned_rigid_bodies.contains(&rigid_body_handle) {
                continue;
            }

            //rigid_body.set_body_type(rapier2d::prelude::RigidBodyType::KinematicPositionBased, false);
        }
        
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
        //println!("time: {:?}", self.);
        
        for (rigid_body_handle, rigid_body) in self.rigid_body_set.iter_mut() {
            if owned_rigid_bodies.contains(&rigid_body_handle) {
                continue;
            }

            let rigid_body_before = rigid_body_set_before.get(rigid_body_handle).expect("Unable to find old version of rigid body before it was updated");

            // we should probably remove this instead of cloning?
            *rigid_body = rigid_body_before.clone();
         
        }

        for (collider_handle, _collider) in self.collider_set.iter_mut() {
            if owned_colliders.contains(&collider_handle) {
                continue;
            }

            let _collider_before = collider_set_before.get(collider_handle).expect("Unable to find old version of collider before it was updated");

            // we should probably remove this instead of cloning?
            //*collider = collider_before.clone();
        }

    }
    
}

#[derive(Serialize, Deserialize)]
pub struct SpaceDiff {
    // for some reason i cant use RigidBodySetDiff directly
    rigid_body_set: Option<<RigidBodySet as Diff>::Repr>,
    collider_set: Option<<ColliderSet as Diff>::Repr>,
    gravity: Option<nalgebra::Matrix<f32, nalgebra::Const<2>, nalgebra::Const<1>, nalgebra::ArrayStorage<f32, 2, 1>>>,
    //broad_phase: Option<BroadPhaseMultiSap>
    // might wanna add the rest of the fields
}

impl Diff for Space {
    type Repr = SpaceDiff; 

    fn diff(&self, other: &Self) -> Self::Repr {
        let mut diff = SpaceDiff {
            rigid_body_set: None,
            collider_set: None,
            gravity: None,
            //broad_phase: None
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

        // if other.broad_phase != self.broad_phase {
        //     diff.broad_phase = Some(other.broad_phase.clone())
        // }


        // if other.island_manager != self.island_manager {
        //     diff.island_manager = Some(other.island_manager.clone());
        // }

        // if self.rigid_body_set.len() != other.rigid_body_set.len() {
        //     fs::write("diff.yaml", bitcode::serialize(&diff).unwrap()).unwrap();
        // }

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

        // if let Some(broad_phase) = &diff.broad_phase {
        //     self.broad_phase = broad_phase.clone()
        // }

        // if let Some(island_manager) = &diff.island_manager {
        //     self.island_manager = island_manager.clone();
        // }
    }

    fn identity() -> Self {
        Space::new()
    }
}