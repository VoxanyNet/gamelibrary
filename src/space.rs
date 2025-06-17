use std::{collections::{HashMap, HashSet}, hash::Hash, time::{Duration, Instant}};

use diff::{Diff, VecDiff};
use nalgebra::{vector, Isometry2, Point2, Vector2};
use rapier2d::{crossbeam::{self, channel::Receiver}, dynamics::{CCDSolver, ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet, RigidBodyHandle, RigidBodySet}, geometry::{ColliderHandle, ColliderSet, DefaultBroadPhase, NarrowPhase}, pipeline::{PhysicsPipeline, QueryPipeline}, prelude::{ChannelEventCollector, Collider, ColliderBuilder, CollisionEvent, GenericJoint, GenericJointBuilder, ImpulseJoint, ImpulseJointHandle, InteractionGroups, RigidBody, RigidBodyBuilder, RigidBodyType, SharedShape}};
use serde::{Deserialize, Deserializer, Serialize};


#[derive(Serialize, Deserialize, Hash, Clone, Copy, PartialEq, Eq, diff::Diff, Debug)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]
pub struct SyncRigidBodyHandle {
    id: u64
}

impl SyncRigidBodyHandle {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().as_u64_pair().0
        }
    }
}

#[derive(Serialize, Deserialize, Hash, Clone, Copy, PartialEq, Eq, diff::Diff, Debug)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]
pub struct SyncColliderHandle {
    id: u64
}

impl SyncColliderHandle {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().as_u64_pair().0
        }
    }
}

// wrapper around RigidBodySet to use SyncHandles which are the same between clients
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct SyncRigidBodySet {
    pub rigid_body_set: RigidBodySet,
    pub sync_map: HashMap<SyncRigidBodyHandle, RigidBodyHandle>,
    pub reverse_sync_map: HashMap<RigidBodyHandle, SyncRigidBodyHandle>
}

impl SyncRigidBodySet {

    pub fn new() -> Self {
        Self {
            rigid_body_set: RigidBodySet::new(),
            sync_map: HashMap::new(),
            reverse_sync_map: HashMap::new()
        }
    }

    pub fn get_local_handle(&self, sync_handle: SyncRigidBodyHandle) -> RigidBodyHandle {
        self.sync_map.get(&sync_handle).unwrap().clone()
    }

    pub fn get_sync_handle(&self, local_handle: RigidBodyHandle) -> SyncRigidBodyHandle {
        self.reverse_sync_map.get(&local_handle).unwrap().clone()
    }

    pub fn get_local(&self, handle: RigidBodyHandle) -> Option<&RigidBody> {
        self.rigid_body_set.get(handle)
    }
    
    pub fn get_local_mut(&mut self, handle: RigidBodyHandle) -> Option<&mut RigidBody> {
        self.rigid_body_set.get_mut(handle)
    }

    pub fn insert_sync(&mut self, rb: impl Into<RigidBody>) -> SyncRigidBodyHandle {

        let sync_handle = SyncRigidBodyHandle::new();

        let local_handle = self.rigid_body_set.insert(rb.into());

        self.sync_map.insert(sync_handle, local_handle);
        self.reverse_sync_map.insert(local_handle, sync_handle);

        sync_handle
    }

    pub fn insert_sync_known_handle(&mut self, rb: impl Into<RigidBody>, sync_handle: SyncRigidBodyHandle) -> SyncRigidBodyHandle {
        
        let local_handle = self.rigid_body_set.insert(rb.into());

        self.sync_map.insert(sync_handle, local_handle);
        self.reverse_sync_map.insert(local_handle, sync_handle);

        sync_handle
        
    }

    pub fn remove_sync(
        &mut self, 
        handle: SyncRigidBodyHandle,
        islands: &mut IslandManager,
        colliders: &mut SyncColliderSet,
        impulse_joints:&mut SyncImpulseJointSet,
        multibody_joints:&mut MultibodyJointSet,
        remove_attached_colliders: bool  
    ) -> Option<RigidBody> {
        match self.sync_map.remove(&handle) {

            
            Some(local_rigid_body_handle) => {

                self.reverse_sync_map.remove(&local_rigid_body_handle).unwrap();

                // caden was here 

                if remove_attached_colliders {

                    let rigid_body = self.rigid_body_set.get(local_rigid_body_handle).unwrap();
                    for local_collider_handle in rigid_body.colliders() {
                        let sync_handle = colliders.get_sync_handle(*local_collider_handle);

                        colliders.sync_map.remove(&sync_handle);
                        colliders.reverse_sync_map.remove(local_collider_handle);
                    }

                    
                };                

                // joints attached to the rigid body are removed so we need to remove them from the sync map
                let mut joint_handles: Vec<ImpulseJointHandle> = Vec::new();
                for (handle, joint) in impulse_joints.impulse_joint_set.iter() {
                    if joint.body1 == local_rigid_body_handle || joint.body2 == local_rigid_body_handle {
                        joint_handles.push(handle);
                    }
                }

                for handle in joint_handles {

                    println!("removing handle: {:?}", handle);

                    let sync_handle = impulse_joints.reverse_sync_map.remove(&handle).unwrap();

                    impulse_joints.sync_map.remove(&sync_handle);
                }

                self.rigid_body_set.remove(local_rigid_body_handle, islands, &mut colliders.collider_set, &mut impulse_joints.impulse_joint_set, multibody_joints, remove_attached_colliders)

                


                
            },
            None => {

                None
            },
        }
    }

    pub fn get_sync_mut(&mut self, handle: SyncRigidBodyHandle) -> Option<&mut RigidBody> {

        match self.sync_map.get(&handle) {
            Some(local_handle) => {
                self.rigid_body_set.get_mut(*local_handle)
            },
            None => {
                None
            },
        }
    }

    pub fn get_sync(&self, handle: SyncRigidBodyHandle) -> Option<&RigidBody> {

        match self.sync_map.get(&handle) {
            Some(local_handle) => {
                self.rigid_body_set.get(*local_handle)
            },
            None => {
                None
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct SyncColliderSet {
    pub collider_set: ColliderSet,
    pub sync_map: HashMap<SyncColliderHandle, ColliderHandle>,
    pub reverse_sync_map: HashMap<ColliderHandle, SyncColliderHandle>
}

impl SyncColliderSet {

    pub fn new() -> Self {
        Self {
            collider_set: ColliderSet::new(),
            sync_map: HashMap::new(),
            reverse_sync_map: HashMap::new()
        }
    }

    pub fn get_local_handle(&self, sync_handle: SyncColliderHandle) -> ColliderHandle {
        self.sync_map.get(&sync_handle).unwrap().clone()
    }

    pub fn get_sync_handle(&self, local_handle: ColliderHandle) -> SyncColliderHandle {
        self.reverse_sync_map.get(&local_handle).unwrap().clone()
    }
    pub fn insert_sync(&mut self, coll: impl Into<Collider>) -> SyncColliderHandle {
        
        let sync_handle = SyncColliderHandle::new();

        let local_handle = self.collider_set.insert(coll.into());

        self.sync_map.insert(sync_handle, local_handle);
        self.reverse_sync_map.insert(local_handle, sync_handle);

        sync_handle
    }

    pub fn get_local(&self, handle: ColliderHandle) -> Option<&Collider> {
        self.collider_set.get(handle)
    }

    pub fn get_local_mut(&mut self, handle: ColliderHandle) -> Option<&mut Collider> {
        self.collider_set.get_mut(handle)
    }

    pub fn insert_sync_known_handle(&mut self, coll: impl Into<Collider>, sync_handle: SyncColliderHandle) -> SyncColliderHandle {
        let local_handle = self.collider_set.insert(coll.into());

        self.sync_map.insert(sync_handle, local_handle);
        self.reverse_sync_map.insert(local_handle, sync_handle);

        sync_handle

    }

    pub fn remove_sync(
        &mut self, 
        handle: SyncColliderHandle, 
        islands: &mut IslandManager, 
        bodies:&mut RigidBodySet,
        wake_up: bool
     ) -> Option<Collider> {
        match self.sync_map.remove(&handle) {
            Some(local_collider_handle) => {

                self.reverse_sync_map.remove(&local_collider_handle).unwrap();

                let collider = self.collider_set.remove(local_collider_handle, islands, bodies, wake_up);

                collider
            },
            None => {
                None
            }
        }

    }

    pub fn insert_with_parent_sync(&mut self, coll: impl Into<Collider>,  sync_body_handle: SyncRigidBodyHandle, bodies: &mut SyncRigidBodySet) -> SyncColliderHandle {

        let local_rigid_body_handle = bodies.sync_map.get(&sync_body_handle).unwrap();

        let sync_handle = self.insert_sync(coll);

        let local_handle = self.sync_map.get(&sync_handle).unwrap();

        self.collider_set.set_parent(*local_handle, Some(*local_rigid_body_handle), &mut bodies.rigid_body_set);

        sync_handle
    }

    pub fn get_sync_mut(&mut self, handle: SyncColliderHandle) -> Option<&mut Collider> {
        match self.sync_map.get(&handle) {
            Some(local_handle) => {
                self.collider_set.get_mut(*local_handle)
            },
            None => {
                None
            },
        }
    }

    pub fn get_sync(&self, handle: SyncColliderHandle) -> Option<&Collider> {
        match self.sync_map.get(&handle) {
            Some(local_handle) => {
                self.collider_set.get(*local_handle)
            },
            None => {
                None
            },
        }
    }

}

#[derive(Serialize, Deserialize, Diff, Clone, PartialEq, Hash, Eq, Copy, Debug)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]
pub struct SyncImpulseJointHandle {
    id: u64
}

impl SyncImpulseJointHandle {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().as_u64_pair().0
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SyncImpulseJointSet {
    pub impulse_joint_set: ImpulseJointSet,
    sync_map: HashMap<SyncImpulseJointHandle, ImpulseJointHandle>,
    reverse_sync_map: HashMap<ImpulseJointHandle, SyncImpulseJointHandle>
}

impl SyncImpulseJointSet {

    pub fn new() -> Self {
        Self {
            impulse_joint_set: ImpulseJointSet::new(),
            sync_map: HashMap::new(),
            reverse_sync_map: HashMap::new(),
        }
    }
    pub fn insert_sync(
        &mut self, 
        body1: RigidBodyHandle, 
        body2: RigidBodyHandle, 
        data: impl Into<GenericJoint>,
        wake_up: bool
    ) -> SyncImpulseJointHandle {
        let sync_handle = SyncImpulseJointHandle::new();

        let local_handle = self.impulse_joint_set.insert(
            body1, 
            body2, 
            data, 
            wake_up
        );

        self.sync_map.insert(sync_handle, local_handle);

        self.reverse_sync_map.insert(local_handle, sync_handle);

        sync_handle
    }

    pub fn remove(
        &mut self,
        handle: SyncImpulseJointHandle
    ) -> Option<ImpulseJoint> {

        match self.sync_map.get(&handle) {
            Some(local_handle) => {

                self.reverse_sync_map.remove(local_handle);

                self.impulse_joint_set.remove(*local_handle, true)


            },
            None => None,
        }

    }

    pub fn insert_sync_known_handle(
        &mut self,
        body1: RigidBodyHandle,
        body2: RigidBodyHandle,
        data: impl Into<GenericJoint>,
        wake_up: bool,
        sync_handle: SyncImpulseJointHandle
    ) -> SyncImpulseJointHandle {

        let local_handle = self.impulse_joint_set.insert(
            body1, 
            body2, 
            data, 
            wake_up
        );

        self.sync_map.insert(sync_handle, local_handle);

        self.reverse_sync_map.insert(local_handle, sync_handle);

        sync_handle
    }

    pub fn get_sync_mut(&mut self, sync_handle: SyncImpulseJointHandle) -> Option<&mut ImpulseJoint> {
        match self.sync_map.get(&sync_handle) {
            Some(local_handle) => {
                self.impulse_joint_set.get_mut(*local_handle)
            },
            None => None,
        }
    }

    pub fn get_sync(&self, sync_handle: SyncImpulseJointHandle) -> Option<&ImpulseJoint> {
        match self.sync_map.get(&sync_handle) {
            Some(local_handle) => {
                self.impulse_joint_set.get(*local_handle)
            },
            None => None
        }
    }

    pub fn remove_sync(&mut self, sync_handle: SyncImpulseJointHandle, wake_up: bool) -> Option<ImpulseJoint> {
        match self.sync_map.remove(&sync_handle) {
            Some(local_handle) => {
                self.impulse_joint_set.remove(local_handle, wake_up)

            },
            None => None,
        }
    }

    
}

#[derive(Serialize)]
pub struct Space {
    
    pub sync_rigid_body_set: SyncRigidBodySet,
    #[serde(skip)]
    pub collision_recv: Receiver<CollisionEvent>,
    pub sync_collider_set: SyncColliderSet,
    pub gravity: nalgebra::Matrix<f32, nalgebra::Const<2>, nalgebra::Const<1>, nalgebra::ArrayStorage<f32, 2, 1>>,
    pub integration_parameters: IntegrationParameters,
    #[serde(skip)]
    pub physics_pipeline: PhysicsPipeline,
    pub island_manager: IslandManager,
    pub broad_phase: DefaultBroadPhase,
    pub narrow_phase: NarrowPhase,
    pub sync_impulse_joint_set: SyncImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
    pub query_pipeline: QueryPipeline,
    pub physics_hooks: (),
    #[serde(skip)]
    pub event_handler: ChannelEventCollector,
    #[serde(skip)]
    pub last_step: Instant,
    #[serde(skip)]
    pub owned_rigid_bodies: Vec<SyncRigidBodyHandle>,
    #[serde(skip)]
    pub owned_colliders: Vec<SyncColliderHandle>,
    #[serde(skip)]
    pub owned_joints: Vec<SyncImpulseJointHandle>
}

impl<'de> Deserialize<'de> for Space {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct SpaceHelper {
            sync_rigid_body_set: SyncRigidBodySet,
            sync_collider_set: SyncColliderSet,
            gravity: nalgebra::Matrix<f32, nalgebra::Const<2>, nalgebra::Const<1>, nalgebra::ArrayStorage<f32, 2, 1>>,
            integration_parameters: IntegrationParameters,
            island_manager: IslandManager,
            broad_phase: DefaultBroadPhase,
            narrow_phase: NarrowPhase,
            sync_impulse_joint_set: SyncImpulseJointSet,
            multibody_joint_set: MultibodyJointSet,
            ccd_solver: CCDSolver,
            query_pipeline: QueryPipeline
            
        }

        let helper = SpaceHelper::deserialize(deserializer)?;

        let (collision_send, collision_recv) = crossbeam::channel::unbounded();
        let (contact_force_send, _contact_force_recv) = crossbeam::channel::unbounded();
        let event_handler = ChannelEventCollector::new(collision_send, contact_force_send);

        Ok(Space {
            sync_rigid_body_set: helper.sync_rigid_body_set,
            collision_recv,
            sync_collider_set: helper.sync_collider_set,
            gravity: helper.gravity,
            integration_parameters: helper.integration_parameters,
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: helper.island_manager,
            broad_phase: helper.broad_phase,
            narrow_phase: helper.narrow_phase,
            sync_impulse_joint_set: helper.sync_impulse_joint_set,
            multibody_joint_set: helper.multibody_joint_set,
            ccd_solver: helper.ccd_solver,
            query_pipeline: helper.query_pipeline,
            event_handler,
            physics_hooks: (),
            last_step: Instant::now(),
            owned_colliders: Vec::new(),
            owned_rigid_bodies: Vec::new(),
            owned_joints: Vec::new()
        })
    }
}

impl Clone for Space {
    fn clone(&self) -> Self {

        let (collision_send, collision_recv) = crossbeam::channel::unbounded();
        let (contact_force_send, _contact_force_recv) = crossbeam::channel::unbounded();
        let event_handler = ChannelEventCollector::new(collision_send, contact_force_send);

        Self {
            sync_rigid_body_set: self.sync_rigid_body_set.clone(),
            sync_collider_set: self.sync_collider_set.clone(),
            gravity: self.gravity.clone(),
            integration_parameters: self.integration_parameters.clone(),
            physics_pipeline: self.physics_pipeline.clone(),
            island_manager: self.island_manager.clone(),
            broad_phase: self.broad_phase.clone(),
            narrow_phase: self.narrow_phase.clone(),
            sync_impulse_joint_set: self.sync_impulse_joint_set.clone(),
            multibody_joint_set: self.multibody_joint_set.clone(),
            ccd_solver: self.ccd_solver.clone(),
            query_pipeline: self.query_pipeline.clone(),
            physics_hooks: self.physics_hooks.clone(),
            event_handler,
            collision_recv,
            last_step: Instant::now(),
            owned_colliders: self.owned_colliders.clone(),
            owned_rigid_bodies: self.owned_rigid_bodies.clone(),
            owned_joints: self.owned_joints.clone()
        }
    }
}

impl PartialEq for Space {
    fn eq(&self, other: &Self) -> bool {
        other.sync_rigid_body_set == self.sync_rigid_body_set && other.sync_collider_set == self.sync_collider_set
    }

    fn ne(&self, other: &Self) -> bool {
        other.sync_rigid_body_set != self.sync_rigid_body_set || other.sync_collider_set != self.sync_collider_set
    }
}

impl Space {

    pub fn new() -> Self {
        let sync_rigid_body_set = SyncRigidBodySet::new();
        let sync_collider_set = SyncColliderSet::new();


        let (collision_send, collision_recv) = crossbeam::channel::unbounded();
        let (contact_force_send, _contact_force_recv) = crossbeam::channel::unbounded();
        let event_handler = ChannelEventCollector::new(collision_send, contact_force_send);
    
        /* Create other structures necessary for the simulation. */
        let gravity = vector![0.0, 0.];
        let mut integration_parameters = IntegrationParameters::default();

        integration_parameters.max_ccd_substeps = 100;
        let physics_pipeline = PhysicsPipeline::new();
        let island_manager = IslandManager::new();
        let broad_phase = DefaultBroadPhase::new();
        let narrow_phase = NarrowPhase::new();
        let sync_impulse_joint_set = SyncImpulseJointSet::new();
        let multibody_joint_set = MultibodyJointSet::new();
        let ccd_solver = CCDSolver::new();
        let query_pipeline = QueryPipeline::new();
        let physics_hooks = ();
        let last_step = Instant::now();

        Self { 
            sync_rigid_body_set, 
            sync_collider_set, 
            gravity, 
            integration_parameters, 
            physics_pipeline, 
            island_manager, 
            broad_phase, 
            narrow_phase, 
            sync_impulse_joint_set, 
            multibody_joint_set, 
            ccd_solver, 
            query_pipeline, 
            physics_hooks, 
            event_handler,
            collision_recv,
            last_step,
            owned_colliders: Vec::new(),
            owned_rigid_bodies: Vec::new(),
            owned_joints: vec![]
        }
    }



    

    pub fn step(&mut self, owned_rigid_bodies: &Vec<SyncRigidBodyHandle>, owned_colliders: &Vec<SyncColliderHandle>, owned_joints: &Vec<SyncImpulseJointHandle>, dt: &Instant) {

        self.owned_rigid_bodies = owned_rigid_bodies.clone();
        self.owned_colliders = owned_colliders.clone();
        self.owned_joints = owned_joints.clone();

        self.last_step = Instant::now();

        self.integration_parameters.dt = dt.elapsed().as_secs_f32();
        

        for (rigid_body_handle, rigid_body) in self.sync_rigid_body_set.rigid_body_set.iter_mut() {

            let sync_rigid_body_handle = self.sync_rigid_body_set.reverse_sync_map.get(&rigid_body_handle).unwrap();

            if owned_rigid_bodies.contains(sync_rigid_body_handle) {
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
            &mut self.sync_rigid_body_set.rigid_body_set,
            &mut self.sync_collider_set.collider_set,
            &mut self.sync_impulse_joint_set.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &self.physics_hooks,
            &self.event_handler
        );
        //println!("time: {:?}", self.);
        
        // for (rigid_body_handle, rigid_body) in self.sync_rigid_body_set.rigid_body_set.iter_mut() {

        //     let sync_rigid_body_handle = self.sync_rigid_body_set.reverse_sync_map.get(&rigid_body_handle).unwrap();

        //     if owned_rigid_bodies.contains(sync_rigid_body_handle) {
        //         continue;
        //     }

        //     let rigid_body_before = rigid_body_set_before.get(rigid_body_handle).expect("Unable to find old version of rigid body before it was updated");

        //     // we should probably remove this instead of cloning?
        //     *rigid_body = rigid_body_before.clone();
         
        // }

        // for (collider_handle, _collider) in self.sync_collider_set.collider_set.iter_mut() {

        //     let sync_collider_handle = self.sync_collider_set.reverse_sync_map.get(&collider_handle).unwrap();

        //     if owned_colliders.contains(sync_collider_handle) {
        //         continue;
        //     }

        //     let _collider_before = collider_set_before.get(collider_handle).expect("Unable to find old version of collider before it was updated");

        //     // we should probably remove this instead of cloning?
        //     //*collider = collider_before.clone();
        // }

    }
    
}


#[derive(Serialize, Deserialize)]
pub struct SpaceDiff {
    sync_rigid_body_set: SyncRigidBodySetDiff,
    sync_collider_set: SyncColliderSetDiff,
    sync_impulse_joint_set: SyncImpulseJointSetDiff,
    gravity: Option<nalgebra::Matrix<f32, nalgebra::Const<2>, nalgebra::Const<1>, nalgebra::ArrayStorage<f32, 2, 1>>>,
    //broad_phase: Option<BroadPhaseMultiSap>
    // might wanna add the rest of the fields
}


#[derive(Serialize, Deserialize)]
pub struct RigidBodyDiff {
    pub position: Option<Isometry2<f32>>,
    pub velocity: Option<Vector2<f32>>,
    pub angular_velocity: Option<f32>,
    // consider adding RigidBodyForces here! and other stuff
    pub colliders: Option<VecDiff<SyncColliderHandle>>,
    pub body_type: Option<RigidBodyType>,
    pub mass: Option<f32>
}

#[derive(Serialize, Deserialize)]
pub struct ColliderDiff {
    pub shape: Option<SharedShape>,
    pub parent: Option<SyncRigidBodyHandle>, // need to add position relative to parent
    pub position: Option<Isometry2<f32>>,
    pub collision_groups: Option<InteractionGroups>,
    pub mass: Option<f32>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImpulseJointDiff {
    pub local_anchor_1: Option<Point2<f32>>,
    pub local_anchor_2: Option<Point2<f32>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewSyncImpulseJoint {
    joint_data: GenericJoint,
    body_handle_1: SyncRigidBodyHandle,
    body_handle_2: SyncRigidBodyHandle
}

#[derive(Serialize, Deserialize)]
pub struct SyncImpulseJointSetDiff {
    altered: HashMap<SyncImpulseJointHandle, ImpulseJointDiff>,
    new: HashMap<SyncImpulseJointHandle, NewSyncImpulseJoint>,
    removed: HashSet<SyncImpulseJointHandle>
}

impl SyncImpulseJointSetDiff {
    pub fn new() -> Self {
        Self {
            altered: HashMap::new(),
            removed: HashSet::new(),
            new: HashMap::new()
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SyncRigidBodySetDiff {
    altered: HashMap<SyncRigidBodyHandle, RigidBodyDiff>,
    removed: HashSet<SyncRigidBodyHandle>
}

impl SyncRigidBodySetDiff {
    pub fn new() -> Self {
        Self {
            altered: HashMap::new(),
            removed: HashSet::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SyncColliderSetDiff {
    altered: HashMap<SyncColliderHandle, ColliderDiff>,
    removed: HashSet<SyncColliderHandle>
}

impl SyncColliderSetDiff {
    pub fn new() -> Self {
        Self {
            altered: HashMap::new(),
            removed: HashSet::new(),
        }
    }
}
impl Diff for Space {
    type Repr = SpaceDiff; 

    fn diff(&self, other: &Self) -> Self::Repr {
        let mut diff = SpaceDiff {
            sync_impulse_joint_set: SyncImpulseJointSetDiff::new(),
            sync_rigid_body_set: SyncRigidBodySetDiff::new(),
            sync_collider_set: SyncColliderSetDiff::new(),
            gravity: None,
        };

        // RIGID BODIES
        if other.sync_rigid_body_set.rigid_body_set != self.sync_rigid_body_set.rigid_body_set {
            for (sync_rigid_body_handle, local_rigid_body_handle) in &self.sync_rigid_body_set.sync_map {
                
                // we dont want to create a diff for a rigid body we dont control
                if other.owned_rigid_bodies.contains(sync_rigid_body_handle) == false {
                    continue;
                }

                match other.sync_rigid_body_set.sync_map.get(&sync_rigid_body_handle) {
                    
                    // the rigid body in in both Spaces
                    Some(other_local_rigid_body_handle) => {
                        
                        // we can just fetch the rigid body using the local handle i think it is faster this way
                        let rigid_body = self.sync_rigid_body_set.rigid_body_set.get(*local_rigid_body_handle).unwrap();

                        // i dont think we technically need to use other_local_rigid_body because the local handle should not change for a given sync handle
                        let other_rigid_body  = other.sync_rigid_body_set.rigid_body_set.get(*other_local_rigid_body_handle).unwrap();
                        
                        // actually do the diff 
                        if other_rigid_body != rigid_body {

                            let mut rigid_body_diff = RigidBodyDiff {
                                position: None,
                                velocity: None,
                                angular_velocity: None,
                                colliders: None,
                                body_type: None,
                                mass: None
                            };

                            
                            if other_rigid_body.position() != rigid_body.position() {

                                //println!("{:?} changed its position to: x: {:?}, y: {:?}", sync_rigid_body_handle, other_rigid_body.position().translation.x, other_rigid_body.position().translation.y);
                                rigid_body_diff.position = Some(*other_rigid_body.position());
                            }

                            if other_rigid_body.linvel() != rigid_body.linvel() {
                                rigid_body_diff.velocity = Some(*other_rigid_body.linvel());
                            }

                            if other_rigid_body.mass() != rigid_body.mass() {
                                rigid_body_diff.mass = Some(other_rigid_body.mass());
                            }

                            if other_rigid_body.angvel() != rigid_body.angvel() {
                                rigid_body_diff.angular_velocity = Some(other_rigid_body.angvel());
                            }

                            if other_rigid_body.colliders() != rigid_body.colliders() {
                                // we want to create a vec diff of sync collider handles, this is certainly one way to do it!
                                let mut sync_collider_handles: Vec<SyncColliderHandle> = Vec::new();
                                let mut other_sync_collider_handles: Vec<SyncColliderHandle> = Vec::new();

                                // convert the collider handles into sync collider handles
                                for collider_handle in rigid_body.colliders() {
                                    let sync_collider_handle = self.sync_collider_set.reverse_sync_map.get(collider_handle).unwrap();

                                    sync_collider_handles.push(*sync_collider_handle);
                                }

                                for other_collider_handle in other_rigid_body.colliders() {
                                    let other_sync_collider_handle = other.sync_collider_set.reverse_sync_map.get(other_collider_handle).unwrap();

                                    other_sync_collider_handles.push(*other_sync_collider_handle);
                                }

                                rigid_body_diff.colliders = Some(sync_collider_handles.diff(&other_sync_collider_handles));


                            }
                            
                            if other_rigid_body.body_type() != rigid_body.body_type() {
                                rigid_body_diff.body_type = Some(other_rigid_body.body_type())
                            }

                            diff.sync_rigid_body_set.altered.insert(*sync_rigid_body_handle, rigid_body_diff);
                        }
                    },
                    
                    // rigid body has been removed
                    None => {

                        println!("{:?} has been removed", sync_rigid_body_handle);

                        diff.sync_rigid_body_set.removed.insert(*sync_rigid_body_handle);
                    },
                }
            }

            for (other_sync_rigid_body_handle, other_local_rigid_body_handle) in &other.sync_rigid_body_set.sync_map {

                // we dont need to check for body ownership when we are creating NEW bodies

                match self.sync_rigid_body_set.sync_map.get(&other_sync_rigid_body_handle) {
                    // item is in both Spaces (already handled)
                    Some(_) => {},

                    // item is not in the old Space so we must add it
                    // its NEW
                    None => {

                        let other_rigid_body = other.sync_rigid_body_set.rigid_body_set.get(*other_local_rigid_body_handle).unwrap();

                        // need to make the sync collider handles here too!
                        let mut sync_collider_handles: Vec<SyncColliderHandle> = Vec::new();
                        let mut other_sync_collider_handles: Vec<SyncColliderHandle> = Vec::new();


                        for other_collider_handle in other_rigid_body.colliders() {
                            let other_sync_collider_handle = other.sync_collider_set.reverse_sync_map.get(other_collider_handle).unwrap();

                            other_sync_collider_handles.push(*other_sync_collider_handle);
                        }

                    
                        let rigid_body_diff = RigidBodyDiff {
                            position:  Some(*other_rigid_body.position()),
                            velocity: Some(*other_rigid_body.linvel()),
                            angular_velocity: Some(other_rigid_body.angvel()),
                            colliders: Some(sync_collider_handles.diff(&other_sync_collider_handles)),
                            body_type: Some(other_rigid_body.body_type()),
                            mass: Some(other_rigid_body.mass())
                        };

                        diff.sync_rigid_body_set.altered.insert(
                            *other_sync_rigid_body_handle, 
                            rigid_body_diff
                        );
                    },
                }
            }
        }
        
        // COLLIDERS
        if other.sync_collider_set.collider_set != self.sync_collider_set.collider_set {
            for (sync_collider_handle, local_collider_handle) in &self.sync_collider_set.sync_map {

                // dont update colliders we don't own
                if self.owned_colliders.contains(sync_collider_handle) == false {
                    continue;
                }
    
                match other.sync_collider_set.sync_map.get(&sync_collider_handle) {
                    
                    // the collider is in both Spaces
                    Some(other_collider_handle) => {
                        let collider = self.sync_collider_set.collider_set.get(*local_collider_handle).unwrap();

                        let other_collider = other.sync_collider_set.collider_set.get(*other_collider_handle).unwrap();

                        if other_collider != collider {
                            let mut collider_diff = ColliderDiff {
                                shape: None,
                                parent: None,
                                position: None,
                                collision_groups: None,
                                mass: None
                            };

                            if other_collider.collision_groups() != collider.collision_groups() {
                                collider_diff.collision_groups = Some(other_collider.collision_groups())
                            }

                            if other_collider.mass() != collider.mass() {
                                collider_diff.mass = Some(other_collider.mass());
                            }

                            if other_collider.shared_shape() != collider.shared_shape() {
                                collider_diff.shape = Some(other_collider.shared_shape().clone());
                            }

                            if other_collider.parent() != collider.parent() {
                                if let Some(other_collider_parent) = other_collider.parent() {
                                    let other_sync_collider_parent = other.sync_rigid_body_set.reverse_sync_map.get(&other_collider_parent).unwrap();

                                    collider_diff.parent = Some(*other_sync_collider_parent);
                                }

                                else {
                                    collider_diff.parent = None;
                                }
                            }

                            if other_collider.position() != collider.position() {
                                collider_diff.position = Some(*other_collider.position());
                            }

                            diff.sync_collider_set.altered.insert(*sync_collider_handle, collider_diff);
                        }

                        
                    },
                    None => {
                        diff.sync_collider_set.removed.insert(*sync_collider_handle);
                    },
                }
            }

            for (other_sync_collider_handle, other_local_collider_handle) in &other.sync_collider_set.sync_map {
                match self.sync_collider_set.sync_map.get(&other_sync_collider_handle) {
                    Some(_) => {},
                    None => {

                        println!("new collider!!!");
                        
                        let other_collider = other.sync_collider_set.collider_set.get(*other_local_collider_handle).unwrap();

                        let parent: Option<SyncRigidBodyHandle> = match other_collider.parent() {
                            Some(local_parent_handle) => {
                                other.sync_rigid_body_set.reverse_sync_map.get(&local_parent_handle).cloned()
                            },
                            None => {
                                None
                            },
                        };

                        let collider_diff = ColliderDiff {
                            shape: Some(other_collider.shared_shape().clone()),
                            parent: parent,
                            position: Some(*other_collider.position()),
                            collision_groups: Some(other_collider.collision_groups()),
                            mass: Some(other_collider.mass())
                        };

                        diff.sync_collider_set.altered.insert(*other_sync_collider_handle, collider_diff);
                    },
                }
            }


        }


        // // IMPULSE JOINT SET
        for (sync_joint_handle, local_joint_handle) in &self.sync_impulse_joint_set.sync_map {

            // if self.owned_joints.contains(sync_joint_handle) == false {
            //     continue;
            // }

            match other.sync_impulse_joint_set.sync_map.get(&sync_joint_handle) {
                
                // the joint is in both Spaces (we just need to update it)
                Some(other_local_joint_handle) => {
                    let joint = self.sync_impulse_joint_set.impulse_joint_set.get(*local_joint_handle).unwrap();

                    let other_joint = other.sync_impulse_joint_set.impulse_joint_set.get(*other_local_joint_handle).unwrap();
                    
                    // we can remove this and just check the attributes individually
                    if other_joint != joint {


                        let mut impulse_joint_diff = ImpulseJointDiff {
                            local_anchor_1: None,
                            local_anchor_2: None,
                        };

                        if other_joint.data.local_anchor1() != joint.data.local_anchor1() {

                            println!("updating local anchor 1");

                            impulse_joint_diff.local_anchor_1 = Some(other_joint.data.local_anchor1());
                        }

                        if other_joint.data.local_anchor2() != joint.data.local_anchor2() {
                            impulse_joint_diff.local_anchor_2 = Some(other_joint.data.local_anchor2());

                            println!("updating local anchor 2");
                        }

                        diff.sync_impulse_joint_set.altered.insert(*sync_joint_handle, impulse_joint_diff);
                    }
                },
                None => {
                    diff.sync_impulse_joint_set.removed.insert(*sync_joint_handle);
                },
            }
        }

        for (other_sync_joint_handle, other_local_joint_handle) in &other.sync_impulse_joint_set.sync_map {
            match self.sync_impulse_joint_set.sync_map.get(&other_sync_joint_handle) {
                Some(_) => {},

                // new joint
                None => {

                    println!("NEW JOINT!");

                    let new_joint = other.sync_impulse_joint_set.impulse_joint_set.get(*other_local_joint_handle).unwrap();

                    let body_1_sync_handle = other.sync_rigid_body_set.get_sync_handle(new_joint.body1);
                    let body_2_sync_handle = other.sync_rigid_body_set.get_sync_handle(new_joint.body2);
        
                    let new_sync_impulse_joint = NewSyncImpulseJoint {
                        joint_data: new_joint.data.clone(),
                        body_handle_1: body_1_sync_handle,
                        body_handle_2: body_2_sync_handle,
                    };
                    
                    diff.sync_impulse_joint_set.new.insert(*other_sync_joint_handle, new_sync_impulse_joint); 
                },
            }
        }

        if other.gravity != self.gravity {
            diff.gravity = Some(other.gravity)
        }

        diff

    }

    fn apply(&mut self, diff: &Self::Repr) {
        
        diff.sync_rigid_body_set.removed.iter().for_each(|deleted_sync_rigid_body_handle| {
            
            self.sync_rigid_body_set.remove_sync(
                *deleted_sync_rigid_body_handle,
                &mut self.island_manager,
                &mut self.sync_collider_set,
                &mut self.sync_impulse_joint_set,
                &mut self.multibody_joint_set,
                false
            );

        });

        diff.sync_collider_set.removed.iter().for_each(|deleted_sync_collider_handle| {

            println!("removing collider: {:?}", deleted_sync_collider_handle);

            self.sync_collider_set.remove_sync(
                *deleted_sync_collider_handle, 
                &mut self.island_manager, 
                &mut self.sync_rigid_body_set.rigid_body_set, 
                true
            );
        });

        diff.sync_impulse_joint_set.removed.iter().for_each(|deleted_sync_joint_handle| {
            self.sync_impulse_joint_set.remove_sync(*deleted_sync_joint_handle, true);
        });


        for (sync_rigid_body_handle, rigid_body_diff) in &diff.sync_rigid_body_set.altered {

            //println!("APPLY {:?}", sync_rigid_body_handle);
            let rigid_body = match self.sync_rigid_body_set.get_sync_mut(*sync_rigid_body_handle) {
                Some(existing_rigid_body) => existing_rigid_body,
                None => {
                    // need to add new rigid body if it doesnt already exist
                    
                    let mut rigid_body = RigidBodyBuilder::dynamic().build();

                    // rigid_body.lock_translations(true, true);
                    // rigid_body.lock_rotations(true, true);

                    rigid_body.enable_ccd(true);
                    rigid_body.set_soft_ccd_prediction(20.);


                    self.sync_rigid_body_set.insert_sync_known_handle(
                        rigid_body,
                        *sync_rigid_body_handle
                    );

                    self.sync_rigid_body_set.get_sync_mut(*sync_rigid_body_handle).unwrap()

                },
            };

            if let Some(position) = rigid_body_diff.position {
                rigid_body.set_position(position, true);

                //println!("{:?} applied position change to x: {:?}, y: {:?}", sync_rigid_body_handle, position.translation.x, position.translation.y);

            }

            if let Some(mas) = rigid_body_diff.mass {
                rigid_body.set_additional_mass(mas, true);
            }

            if let Some(velocity) = rigid_body_diff.velocity {
                rigid_body.set_linvel(velocity, true);
            }

            if let Some(angular_velocity) = rigid_body_diff.angular_velocity {
                rigid_body.set_angvel(angular_velocity, true);
            }

        }


        // COLLIDER SET
        for (sync_collider_handle, collider_diff) in &diff.sync_collider_set.altered {
            
            
            let collider = match self.sync_collider_set.get_sync_mut(*sync_collider_handle) {
                Some(existing_collider) => {existing_collider},
                None => {

                    // if the collider isnt already in the collider set we create it and attach it to its rigid body
                    let mut collider = ColliderBuilder::cuboid(1., 1.).build();

                    collider.set_position(collider_diff.position.unwrap());

                    collider.set_shape(collider_diff.shape.clone().unwrap());

                    collider.set_mass(collider_diff.mass.unwrap());
                
                    self.sync_collider_set.insert_sync_known_handle(collider, *sync_collider_handle);

                    let local_collider_handle = self.sync_collider_set.sync_map.get(sync_collider_handle).unwrap();

                    // attach the collider to the parent if exists
                    match collider_diff.parent {
                        Some(parent) => {
                            let local_handle = self.sync_rigid_body_set.sync_map.get(&parent).unwrap();

                            self.sync_collider_set.collider_set.set_parent(*local_collider_handle, Some(*local_handle), &mut self.sync_rigid_body_set.rigid_body_set);
                        },
                        None => {},
                    }

                    // can continue to the next collider because we already set all the data
                    continue;
                },
            };

            if let Some(shape) = &collider_diff.shape {
                collider.set_shape(shape.clone());
            };

            if let Some(mass) = collider_diff.mass {
                collider.set_mass(mass);
            }

            if let Some(position) = &collider_diff.position {
                collider.set_position(*position);
            }


        }

        // IMPULSE JOINTS
        for (sync_joint_handle, new_sync_joint) in &diff.sync_impulse_joint_set.new {

            let body1_local_handle = self.sync_rigid_body_set.get_local_handle(new_sync_joint.body_handle_1);
            let body2_local_handle = self.sync_rigid_body_set.get_local_handle(new_sync_joint.body_handle_2);

            
            self.sync_impulse_joint_set.insert_sync_known_handle(
                body1_local_handle, 
                body2_local_handle, 
                new_sync_joint.joint_data, 
                true, 
                *sync_joint_handle
            );

            println!("contacts enabled: {:?}", new_sync_joint.joint_data.contacts_enabled());

        }

        
        for (sync_joint_handle, sync_joint_diff) in &diff.sync_impulse_joint_set.altered {
            let joint = match self.sync_impulse_joint_set.get_sync_mut(*sync_joint_handle) {
                // the joint already exists
                Some(existing_joint) => existing_joint,
                
                // need to add new joint
                None => {   

                    unreachable!()
                    
                },
            };

            if let Some(local_anchor_1) = sync_joint_diff.local_anchor_1 {
                joint.data.set_local_anchor1(local_anchor_1);
            }

            if let Some(local_anchor_2) = sync_joint_diff.local_anchor_2 {
                joint.data.set_local_anchor2(local_anchor_2);
            }
        }
        

        if let Some(gravity) = &diff.gravity {
            self.gravity = *gravity;
        };

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