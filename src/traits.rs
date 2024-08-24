
use macroquad::color::WHITE;
use macroquad::input::{self, is_mouse_button_down, is_mouse_button_pressed};
use macroquad::math::{vec2, Rect, Vec2};
use macroquad::shapes::DrawRectangleParams;
use macroquad::texture::{draw_texture_ex, DrawTextureParams};
use macroquad::window::screen_height;
use nalgebra::{point, vector};
use rapier2d::geometry::ColliderHandle;
use rapier2d::pipeline::QueryFilter;
use rapier2d::prelude::RigidBodyHandle;

use crate::space::Space;
use crate::{rapier_mouse_world_pos, rapier_to_macroquad};
use crate::texture_loader::TextureLoader;

pub trait HasPhysics {
    fn collider_handle(&self) -> &ColliderHandle;
    fn rigid_body_handle(&self) -> &RigidBodyHandle;
    fn selected(&self) -> &bool;
    fn selected_mut(&mut self) -> &mut bool;
    fn dragging(&mut self) -> &mut bool; // structure is currently being dragged
    fn drag_offset(&mut self) -> &mut Option<Vec2>; // when dragging the body, we teleport the body to the mouse plus this offset

    fn contains_point(&mut self, space: &mut Space, point: Vec2) -> bool {
        let mut contains_point: bool = false;

        space.query_pipeline.update(&space.collider_set);

        space.query_pipeline.intersections_with_point(
            &space.rigid_body_set, &space.collider_set, &point![point.x, point.y], QueryFilter::default(), |handle| {
                if *self.collider_handle() == handle {
                    contains_point = true;
                    return false
                }

                return true
            }
        );

        contains_point
    } 

    async fn draw_texture(&self, space: &Space, texture_path: &String, textures: &mut TextureLoader) {
        let rigid_body = space.rigid_body_set.get(*self.rigid_body_handle()).unwrap();
        let collider = space.collider_set.get(*self.collider_handle()).unwrap();

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
                (shape.half_extents.x * 2.) + 10., 
                (shape.half_extents.y * 2.) + 10., 
                DrawRectangleParams { offset: macroquad::math::Vec2::new(0.5, 0.5), rotation: rotation * -1., color: WHITE }
            );
        } 

        draw_texture_ex(
            textures.get(texture_path).await, 
            draw_pos.x - shape.half_extents.x, 
            draw_pos.y - shape.half_extents.y, 
            WHITE, 
            DrawTextureParams {
                dest_size: Some(vec2(shape.half_extents.x * 2., shape.half_extents.y * 2.)),
                source: None,
                rotation: rotation * -1.,
                flip_x: false,
                flip_y: false,
                pivot: None,
            }
        );

        
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
        
        let collider = space.collider_set.get_mut(*self.collider_handle()).unwrap();

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

        space.query_pipeline.intersections_with_point(
            &space.rigid_body_set, &space.collider_set, &point![mouse_pos.x, mouse_pos.y], QueryFilter::default(), |handle| {
                
                if *self.collider_handle() == handle {
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

            let collider = space.collider_set.get(*self.collider_handle()).unwrap();

            match collider.parent() {

                Some(rigid_body_handle) => {
                    let rigid_body = space.rigid_body_set.get(rigid_body_handle).unwrap();

                    *self.drag_offset() = Some(
                        Vec2::new(mouse_pos.x - rigid_body.position().translation.x, mouse_pos.y - rigid_body.position().translation.y)
                    );

                },
                None => {

                    let collider = space.collider_set.get(*self.collider_handle()).unwrap();

                    *self.drag_offset() = Some(
                        Vec2::new(mouse_pos.x - collider.position().translation.x, mouse_pos.y - collider.position().translation.y)
                    );
                },
            }

            
        }

        *self.dragging() = true;

        

    }

    async fn draw_collider(&mut self, space: &Space) {
        let collider_handle = self.collider_handle();
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

