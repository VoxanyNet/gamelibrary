
use macroquad::color::{Color, RED, WHITE};
use macroquad::input::{self, is_key_down, is_mouse_button_down, is_mouse_button_pressed};
use macroquad::math::{vec2, Rect, Vec2};
use macroquad::shapes::{draw_rectangle, draw_rectangle_ex, DrawRectangleParams};
use macroquad::texture::{draw_texture_ex, DrawTextureParams};
use macroquad::window::screen_height;
use nalgebra::{point, vector};
use rapier2d::geometry::ColliderHandle;
use rapier2d::math::Rotation;
use rapier2d::pipeline::QueryFilter;
use rapier2d::prelude::RigidBodyHandle;

use crate::space::{Space, SyncColliderHandle, SyncRigidBodyHandle};
use crate::{rapier_mouse_world_pos, rapier_to_macroquad};
use crate::texture_loader::TextureLoader;

pub fn draw_hitbox(space: &Space, rigid_body_handle: SyncRigidBodyHandle, collider_handle: SyncColliderHandle, color: Color) {
    let rigid_body = space.sync_rigid_body_set.get_sync(rigid_body_handle).unwrap();
    let collider = space.sync_collider_set.get_sync(collider_handle).unwrap();

    let shape = collider.shape().as_cuboid().unwrap();

    let position = collider.position().translation;
    let rotation = rigid_body.rotation().angle();

    let draw_pos = rapier_to_macroquad(&vec2(position.x, position.y));

    macroquad::shapes::draw_rectangle_ex(
        draw_pos.x,
        draw_pos.y, 
        shape.half_extents.x * 2., 
        shape.half_extents.y * 2., 
        DrawRectangleParams { offset: macroquad::math::Vec2::new(0.5, 0.5), rotation: rotation * -1., color }
    );

}

pub async fn draw_texture_onto_physics_body(
    rigid_body_handle: SyncRigidBodyHandle,
    collider_handle: SyncColliderHandle,
    space: &Space, 
    texture_path: &String, 
    textures: &mut TextureLoader, 
    flip_x: bool, 
    flip_y: bool, 
    additional_rotation: f32
) {
    let rigid_body = space.sync_rigid_body_set.get_sync(rigid_body_handle).unwrap();
    let collider = space.sync_collider_set.get_sync(collider_handle).unwrap();

    // use the shape to define how large we should draw the texture
    // maybe we should change this
    let shape = collider.shape().as_cuboid().unwrap();

    let position = rigid_body.position().translation;
    let body_rotation = rigid_body.rotation().angle();

    let draw_pos = rapier_to_macroquad(&vec2(position.x, position.y));

    draw_texture_ex(
        textures.get(texture_path).await, 
        draw_pos.x - shape.half_extents.x, 
        draw_pos.y - shape.half_extents.y, 
        WHITE, 
        DrawTextureParams {
            dest_size: Some(vec2(shape.half_extents.x * 2., shape.half_extents.y * 2.)),
            source: None,
            rotation: (body_rotation * -1.) + additional_rotation,
            flip_x,
            flip_y,
            pivot: None,
        }
    );

    
}

pub trait HasPhysics {
    fn collider_handle(&self) -> &SyncColliderHandle;
    fn rigid_body_handle(&self) -> &SyncRigidBodyHandle;
    fn selected(&self) -> &bool;
    fn selected_mut(&mut self) -> &mut bool;
    fn dragging(&mut self) -> &mut bool; // structure is currently being dragged
    fn drag_offset(&mut self) -> &mut Option<Vec2>; // when dragging the body, we teleport the body to the mouse plus this offset

    fn remove_body_and_collider(&mut self, space: &mut Space) {

        space.sync_rigid_body_set.remove_sync(*self.rigid_body_handle(), &mut space.island_manager, &mut space.sync_collider_set.collider_set, &mut space.impulse_joint_set, &mut space.multibody_joint_set, true);
    }

    fn contains_point(&mut self, space: &mut Space, point: Vec2) -> bool {
        let mut contains_point: bool = false;

        space.query_pipeline.update(&space.sync_collider_set.collider_set);

        let local_collider_handle = space.sync_collider_set.sync_map.get(self.collider_handle()).unwrap();
        space.query_pipeline.intersections_with_point(
            &space.sync_rigid_body_set.rigid_body_set, &space.sync_collider_set.collider_set, &point![point.x, point.y], QueryFilter::default(), |handle| {
                if *local_collider_handle == handle {
                    contains_point = true;
                    return false
                }

                return true
            }
        );

        contains_point
    } 

    fn editor_rotate(&mut self, space: &mut Space) {
        if !*self.selected() {return}

        if !is_key_down(input::KeyCode::R) {return}

        let rigid_body = space.sync_rigid_body_set.get_sync_mut(*self.rigid_body_handle()).unwrap();
        
        rigid_body.set_rotation(Rotation::from_angle(rigid_body.rotation().angle() - 0.05), true);
    }

    fn editor_resize(&mut self, space: &mut Space) {

        if !*self.selected() {
            return;
        }
        let collider = space.sync_collider_set.get_sync_mut(*self.collider_handle()).unwrap();
        let rigid_body = space.sync_rigid_body_set.get_sync_mut(*self.rigid_body_handle()).unwrap();

        let shape = collider.shape_mut().as_cuboid_mut().unwrap();

        let increase_unit = 10.;

        if is_key_down(input::KeyCode::Right) {
            
            shape.half_extents.x += increase_unit;
            rigid_body.set_position(vector![rigid_body.position().translation.x + increase_unit, rigid_body.position().translation.y].into(), true)
        }

        if is_key_down(input::KeyCode::Up) {
            shape.half_extents.y += increase_unit;
            rigid_body.set_position(vector![rigid_body.position().translation.x, rigid_body.position().translation.y + increase_unit].into(), true)
        }

        if is_key_down(input::KeyCode::Down) {
            shape.half_extents.y -= increase_unit;
            rigid_body.set_position(vector![rigid_body.position().translation.x, rigid_body.position().translation.y - increase_unit].into(), true)
        }

        if is_key_down(input::KeyCode::Left) {
            shape.half_extents.x -= increase_unit;
            rigid_body.set_position(vector![rigid_body.position().translation.x - increase_unit, rigid_body.position().translation.y].into(), true)
        }

        if shape.half_extents.x <= 0. {
            shape.half_extents.x = 1.
        }

        if shape.half_extents.y <= 0. {
            shape.half_extents.y = 1.
        }
        
    }

    async fn draw_outline(&self, space: &Space, outline_thickness: f32) {
        let rigid_body = space.sync_rigid_body_set.get_sync(*self.rigid_body_handle()).unwrap();
        let collider = space.sync_collider_set.get_sync(*self.collider_handle()).unwrap();

        // use the shape to define how large we should draw the texture
        // maybe we should change this
        let shape = collider.shape().as_cuboid().unwrap();

        let position = rigid_body.position().translation;
        let rotation = rigid_body.rotation().angle();

        let draw_pos = rapier_to_macroquad(&vec2(position.x, position.y));

        // draw the outline
        if *self.selected() {
            macroquad::shapes::draw_rectangle_ex(
                draw_pos.x,
                draw_pos.y, 
                (shape.half_extents.x * 2.) + outline_thickness, 
                (shape.half_extents.y * 2.) + outline_thickness, 
                DrawRectangleParams { offset: macroquad::math::Vec2::new(0.5, 0.5), rotation: rotation * -1., color: WHITE }
            );
        } 
    }

    fn draw_hitbox(&self, space: &Space) {
        draw_hitbox(space, *self.rigid_body_handle(), *self.collider_handle(), WHITE);

    }
    async fn draw_texture(
        &self, 
        space: &Space, 
        texture_path: &String, 
        textures: &mut TextureLoader, 
        flip_x: bool, 
        flip_y: bool, 
        additional_rotation: f32
    ) {
        draw_texture_onto_physics_body(
            *self.rigid_body_handle(), 
            *self.collider_handle(), 
            space, 
            texture_path, 
            textures, 
            flip_x, 
            flip_y, 
            additional_rotation
        ).await;
        
    }
    
    fn update_selected(&mut self, space: &mut Space, camera_rect: &Rect) {

        if !is_mouse_button_pressed(input::MouseButton::Left) {
            return;
        }

        let mouse_rapier_coords = rapier_mouse_world_pos(camera_rect);

        if self.contains_point(space, mouse_rapier_coords){
            *self.selected_mut() = true;
        }

        else {
            *self.selected_mut() = false;
        }
        
    }

    fn update_drag(&mut self, space: &mut Space, camera_rect: &Rect) {
        // Drag the collider / rigid body with the mouse

        if !*self.dragging() {
            return
        }

        let drag_offset = self.drag_offset().unwrap(); // there shouldn't be a situation where get_dragging returns true and there is no drag offset
        
        let collider = space.sync_collider_set.get_sync_mut(*self.collider_handle()).unwrap();

        let mouse_pos = rapier_mouse_world_pos(camera_rect);

        let offset_mouse_pos = mouse_pos - drag_offset;

        // if the collider has a parent rigid body, we move that instead of the collider
        match &mut collider.parent() {

            Some(rigid_body_handle) => {

                let rigid_body = space.sync_rigid_body_set.rigid_body_set.get_mut(*rigid_body_handle).unwrap();

                rigid_body.set_position(vector![offset_mouse_pos.x, offset_mouse_pos.y].into(), true);

                rigid_body.set_linvel(vector![0., 0.].into(), true);

                collider.set_position(vector![offset_mouse_pos.x, offset_mouse_pos.y].into());


            },
            None => {
                collider.set_position(vector![offset_mouse_pos.x, offset_mouse_pos.y].into());
            },

        }

        

        
    }

    fn update_is_dragging(&mut self, space: &mut Space, camera_rect: &Rect) {
        // Determine if the collider is being dragged

        if !*self.selected() {
            *self.dragging() = false;
            *self.drag_offset() = None;
            return
        }

        if !is_mouse_button_down(input::MouseButton::Left) {
            *self.dragging() = false;
            *self.drag_offset() = None;
            return
        }

        let mouse_pos = rapier_mouse_world_pos(camera_rect);

        // if the body does not contain the mouse, but the button is down, we just dont do anything, because this is still a valid dragging state IF we are already dragging

        let mut contains_mouse = false;

        let local_collider_handle = space.sync_collider_set.sync_map.get(self.collider_handle()).unwrap();
        space.query_pipeline.intersections_with_point(
            &space.sync_rigid_body_set.rigid_body_set, &space.sync_collider_set.collider_set, &point![mouse_pos.x, mouse_pos.y], QueryFilter::default(), |handle| {
                
                
                if *local_collider_handle == handle {
                    contains_mouse = true;
                    return false
                }

                return true
        });

        if !contains_mouse {
            return
        }

        // at this point we know we will update dragging to true, but we want to check if this is a change from the last tick, so that we can set the mouse offset only when we begin dragging
        if !*self.dragging() {

            let collider = space.sync_collider_set.get_sync(*self.collider_handle()).unwrap();

            match collider.parent() {

                Some(rigid_body_handle) => {
                    let rigid_body = space.sync_rigid_body_set.get_local(rigid_body_handle).unwrap();

                    *self.drag_offset() = Some(
                        Vec2::new(mouse_pos.x - rigid_body.position().translation.x, mouse_pos.y - rigid_body.position().translation.y)
                    );

                },
                None => {

                    let collider = space.sync_collider_set.get_sync(*self.collider_handle()).unwrap();

                    *self.drag_offset() = Some(
                        Vec2::new(mouse_pos.x - collider.position().translation.x, mouse_pos.y - collider.position().translation.y)
                    );
                },
            }

            
        }

        *self.dragging() = true;

        

    }

    async fn draw_collider(&self, space: &Space) {
        let collider_handle = self.collider_handle();
        let collider = space.sync_collider_set.get_sync(*collider_handle).expect("Invalid collider handle");

        // if the collider has a rigid body, then we use it's position instead
        let (position, rotation) = match collider.parent() {
            Some(rigid_body_handle) => {
                
                let rigid_body = space.sync_rigid_body_set.get_local(rigid_body_handle).unwrap();

                (rigid_body.position(), rigid_body.rotation())
                

            },
            None => (collider.position(), collider.rotation())
        };

        // get the half extents of the shape. its gotttaa be a squareeee
        let shape = collider.shape().as_typed_shape();

        let (hx, hy) = match shape {
            rapier2d::geometry::TypedShape::Cuboid(cuboid) => {
                (cuboid.half_extents.x, cuboid.half_extents.y)
            },
            _ => panic!("cannot draw non cuboid shape")
        };

        // draw the outline
        if *self.selected() {
            macroquad::shapes::draw_rectangle_ex(
                position.translation.x, 
                ((position.translation.y) * -1.) + screen_height(), 
                (hx * 2.) + 10., 
                (hy * 2.)+ 10., 
                DrawRectangleParams { offset: macroquad::math::Vec2::new(0.5, 0.5), rotation: rotation.angle() * -1., color: WHITE }
            );
        } 

        macroquad::shapes::draw_rectangle_ex(
            position.translation.x, 
            ((position.translation.y) * -1.) + screen_height(), 
            hx * 2., 
            hy * 2., 
            DrawRectangleParams { offset: macroquad::math::Vec2::new(0.5, 0.5), rotation: rotation.angle() * -1., color: WHITE }
        );

        // for resize_handle in self.get_resize_handles() {
        //     // draw the resize handles
        //     macroquad::shapes::draw_rectangle_ex(
        //         position.x, 
        //         position.y, 
        //         resize_handle.w, 
        //         resize_handle.h, 
        //         DrawRectangleParams { offset: macroquad::math::Vec2::new(0.5, 0.5), rotation: rotation * -1., color: ORANGE }
        //     )
        // }
        

    }
}

