use std::collections::HashMap;
use std::time::Instant;

use chrono::TimeDelta;
use macroquad::audio::{self, load_sound};
use macroquad::color::{GREEN, ORANGE, RED, WHITE};
use macroquad::experimental::camera::mouse;
use macroquad::input::{self, is_mouse_button_down, is_mouse_button_pressed, is_mouse_button_released, mouse_position, KeyCode};
use macroquad::math::{Rect, Vec2};
use macroquad::shapes::DrawRectangleParams;
use macroquad::texture::{self, load_texture, Texture2D};
use macroquad::window::screen_height;
use nalgebra::{point, vector, Rotation};
use rapier2d::dynamics::RigidBodyHandle;
use rapier2d::geometry::{Collider, ColliderBuilder, ColliderHandle};
use rapier2d::pipeline::QueryFilter;
use rapier2d::prelude::RigidBodyPosition;

use crate::space::Space;
use crate::{macroquad_to_rapier, rapier_mouse_world_pos, rapier_to_macroquad};

// pub trait HasCollider {
//     fn get_collider(&self) -> Collider;
//     fn set_collider(&mut self, collider: Collider);
// }


pub trait HasCollider {
    fn get_collider_handle(&self) -> &ColliderHandle;
    fn get_selected(&mut self) -> &mut bool;
    fn get_dragging(&mut self) -> &mut bool; // structure is currently being dragged
    fn get_drag_offset(&mut self) -> &mut Option<Vec2>; // when dragging the body, we teleport the body to the mouse plus this offset

    fn contains_point(&mut self, space: &mut Space, point: Vec2) -> bool {
        let mut contains_point: bool = false;

        space.query_pipeline.update(&space.collider_set);

        space.query_pipeline.intersections_with_point(
            &space.rigid_body_set, &space.collider_set, &point![point.x, point.y], QueryFilter::default(), |handle| {
                if *self.get_collider_handle() == handle {
                    contains_point = true;
                    return false
                }

                return true
            }
        );

        contains_point
    } 
    
    fn update_selected(&mut self, space: &mut Space, camera_rect: &Rect) {

        if !is_mouse_button_pressed(input::MouseButton::Left) {
            return;
        }

        let mouse_rapier_coords = rapier_mouse_world_pos(camera_rect);

        if self.contains_point(space, mouse_rapier_coords){
            *self.get_selected() = true;
        }

        else {
            *self.get_selected() = false;
        }
        
    }

    fn update_drag(&mut self, space: &mut Space, camera_rect: &Rect) {
        // Drag the collider / rigid body with the mouse

        if !*self.get_dragging() {
            return
        }

        let drag_offset = self.get_drag_offset().unwrap(); // there shouldn't be a situation where get_dragging returns true and there is no drag offset
        
        let collider = space.collider_set.get_mut(*self.get_collider_handle()).unwrap();

        let mouse_pos = rapier_mouse_world_pos(camera_rect);

        let offset_mouse_pos = mouse_pos - drag_offset;

        // if the collider has a parent rigid body, we move that instead of the collider
        match &mut collider.parent() {

            Some(rigid_body_handle) => {
                let rigid_body = space.rigid_body_set.get_mut(*rigid_body_handle).unwrap();

                rigid_body.set_position(vector![offset_mouse_pos.x, offset_mouse_pos.y].into(), true);

                collider.set_position(vector![offset_mouse_pos.x, offset_mouse_pos.y].into());


            },
            None => {
                collider.set_position(vector![offset_mouse_pos.x, offset_mouse_pos.y].into());
            },
        }

        

        
    }

    fn update_is_dragging(&mut self, space: &mut Space, camera_rect: &Rect) {
        // Determine if the collider is being dragged

        if !*self.get_selected() {
            *self.get_dragging() = false;
            *self.get_drag_offset() = None;
            return
        }

        if !is_mouse_button_down(input::MouseButton::Left) {
            *self.get_dragging() = false;
            *self.get_drag_offset() = None;
            return
        }

        let mouse_pos = rapier_mouse_world_pos(camera_rect);

        // if the body does not contain the mouse, but the button is down, we just dont do anything, because this is still a valid dragging state IF we are already dragging

        let mut contains_mouse = false;

        space.query_pipeline.intersections_with_point(
            &space.rigid_body_set, &space.collider_set, &point![mouse_pos.x, mouse_pos.y], QueryFilter::default(), |handle| {
                
                if *self.get_collider_handle() == handle {
                    contains_mouse = true;
                    return false
                }

                return true
        });

        if !contains_mouse {
            return
        }

        // at this point we know we will update dragging to true, but we want to check if this is a change from the last tick, so that we can set the mouse offset only when we begin dragging
        if !*self.get_dragging() {

            let collider = space.collider_set.get(*self.get_collider_handle()).unwrap();

            match collider.parent() {

                Some(rigid_body_handle) => {
                    let rigid_body = space.rigid_body_set.get(rigid_body_handle).unwrap();

                    *self.get_drag_offset() = Some(
                        Vec2::new(mouse_pos.x - rigid_body.position().translation.x, mouse_pos.y - rigid_body.position().translation.y)
                    );

                },
                None => {

                    let collider = space.collider_set.get(*self.get_collider_handle()).unwrap();

                    *self.get_drag_offset() = Some(
                        Vec2::new(mouse_pos.x - collider.position().translation.x, mouse_pos.y - collider.position().translation.y)
                    );
                },
            }

            
        }

        *self.get_dragging() = true;

        

    }

    async fn draw_collider(&mut self, space: &Space) {
        let collider_handle = self.get_collider_handle();
        let collider = space.collider_set.get(*collider_handle).expect("Invalid collider handle");

        // if the collider has a rigid body, then we use it's position instead
        let (position, rotation) = match collider.parent() {
            Some(rigid_body_handle) => {
                
                let rigid_body = space.rigid_body_set.get(rigid_body_handle).unwrap();

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
        if *self.get_selected() {
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

