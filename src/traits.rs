use std::collections::HashMap;
use std::time::Instant;

use chrono::TimeDelta;
use macroquad::audio::{self, load_sound};
use macroquad::color::{GREEN, ORANGE, RED, WHITE};
use macroquad::input::{self, is_mouse_button_down, is_mouse_button_released, mouse_position};
use macroquad::shapes::DrawRectangleParams;
use macroquad::texture::{self, load_texture, Texture2D};
use macroquad::window::screen_height;
use rapier2d::geometry::{Collider, ColliderBuilder};

use crate::proxies::macroquad::{input::KeyCode, math::{vec2::Vec2, rect::Rect}};
use crate::space::{self, ColliderHandle, RigidBodyHandle, Space};
use crate::{macroquad_to_rapier, rapier_to_macroquad, rigid_body};

pub trait Velocity {
    fn get_velocity(&self) -> Vec2;
    fn set_velocity(&mut self, velocity: Vec2);
}

pub trait Damagable {
    fn damage(&mut self, damage: i32) {
        self.set_health(
            self.get_health() - damage
        )
    }

    fn get_health(&self) -> i32;
    fn set_health(&mut self, health: i32);
}

pub trait Breakable: Damagable + HasRect {
    fn highlight(&mut self) {

        let mouse_pos = Vec2::new(
            macroquad::input::mouse_position().0,
            macroquad::input::mouse_position().1
        );

        if self.get_rect().contains(mouse_pos) {
            self.set_highlighted(true);
        }

        else {
            self.set_highlighted(false);
        }
    }

    fn get_highlighted(&self) -> bool;
    fn set_highlighted(&mut self, highlighted: bool);
}

pub trait Collidable: HasRect + Velocity {

    fn collide(&mut self, collider: &mut dyn Collidable, dt: TimeDelta) {

        // check where our rect will be when it next moves
        let mut next_rect = self.get_rect().clone();

        next_rect.x += self.get_velocity().x * dt.num_milliseconds() as f32;
        next_rect.y += self.get_velocity().y * dt.num_milliseconds() as f32;

        if collider.get_rect().overlaps(&mut next_rect) {
            
            // add our velocity to the collider
            collider.set_velocity(
                collider.get_velocity() + self.get_velocity()
            );

            // invert current velocity
            self.set_velocity(
                self.get_velocity() * -0.05
            );

        }

    }
}

fn round(x: f32) -> f32 {
    f32::trunc(x  * 10.0) / 10.0 // or f32::trunc
}

pub trait Friction: HasRect + Velocity {
    fn apply_friction(&mut self, dt: TimeDelta) {

        self.set_velocity(
            self.get_velocity() + ((-self.get_velocity() * self.friction_coefficient()) * (dt.num_milliseconds() as f32 / 1000.))
        );

        self.set_velocity(
            Vec2::new(round(self.get_velocity().x), round(self.get_velocity().y))
        );
    }

    fn friction_coefficient(&self) -> f32;
}

pub trait Controllable: HasRect + Velocity {
    fn control(&mut self, dt: TimeDelta) {

        let mut velocity = self.get_velocity();
        let acceleration = self.get_acceleration();

        if macroquad::input::is_key_down(self.right_bind().into()) {
            velocity.x += acceleration * dt.num_milliseconds() as f32;
        }

        if macroquad::input::is_key_down(self.left_bind().into()) {
            velocity.x -= acceleration * dt.num_milliseconds() as f32

        }

        if macroquad::input::is_key_down(self.up_bind().into()) {
            velocity.y -= acceleration * dt.num_milliseconds() as f32
        }

        if macroquad::input::is_key_down(self.down_bind().into()) {
            velocity.y += acceleration * dt.num_milliseconds() as f32
        }

        // update to the final velocity
        self.set_velocity(
            velocity
        );

    }

    fn get_acceleration(&self) -> f32;
    fn set_acceleration(&mut self, acceleration: f32);

    fn up_bind(&mut self) -> KeyCode;
    fn down_bind(&mut self) -> KeyCode;
    fn left_bind(&mut self) -> KeyCode;
    fn right_bind(&mut self) -> KeyCode;
}

pub trait Moveable: HasRect + Velocity {
    fn move_by_velocity(&mut self, dt: TimeDelta) {

        let mut rect = self.get_rect();

        //println!("{}", self.get_velocity().x * (dt.num_milliseconds() as f32 / 1000.));

        rect.x += self.get_velocity().x * (dt.num_milliseconds() as f32 / 1000.);
        rect.y += self.get_velocity().y * (dt.num_milliseconds() as f32 / 1000.);

        self.set_rect(rect);
    }
}
pub trait HasRect {
    fn get_rect(&self) -> Rect;
    fn set_rect(&mut self, rect: Rect);
}

// pub trait HasCollider {
//     fn get_collider(&self) -> Collider;
//     fn set_collider(&mut self, collider: Collider);
// }



pub trait Color {
    fn color(&mut self) -> &mut crate::proxies::macroquad::color::Color;
}

pub trait Drawable: HasRect + Color {
    fn draw(&mut self, camera_offset: &Vec2) {
        macroquad::shapes::draw_rectangle(self.get_rect().x, self.get_rect().y, self.get_rect().w, self.get_rect().h, self.color().into());
    }
}

pub struct ResizeHandles {
    top_left: Rect,
    top_right: Rect,
    bottom_left: Rect,
    bottom_right: Rect
}

pub trait HasRigidBody {
    fn get_rigid_body_handle(&self) -> &RigidBodyHandle;
}

pub trait HasCollider: Color {
    fn get_collider_handle(&self) -> &ColliderHandle;
    fn get_selected(&mut self) -> &mut bool;
    fn get_dragging(&mut self) -> &mut bool; // structure is currently being dragged
    fn get_drag_offset(&mut self) -> &mut Option<Vec2>; // when dragging the body, we teleport the body to the mouse plus this offset


    fn update_selected(&mut self, space: &mut Space) {
        if !is_mouse_button_released(input::MouseButton::Left) {
            return;
        }

        if space.query_point(
            macroquad_to_rapier(&Vec2::new(mouse_position().0, mouse_position().1)).into()
        ).contains(self.get_collider_handle()) {
            *self.get_selected() = true;
        }

        else {
            *self.get_selected() = false;
        }
        
    }

    fn update_drag(&mut self, space: &mut Space) {
        if !*self.get_dragging() {
            return
        }

        let drag_offset = self.get_drag_offset().unwrap(); // there shouldn't be a situation where get_dragging returns true and there is no drag offset
        
        let collider = space.get_collider_mut(self.get_collider_handle()).unwrap();


        let mouse_pos = macroquad_to_rapier(
            &Vec2::new(mouse_position().0, mouse_position().1)
        );

        // if the collider has a parent rigid body, we move that instead of the collider
        match collider.clone().parent {

            Some(rigid_body_handle) => {
                let rigid_body = space.get_rigid_body_mut(&rigid_body_handle).unwrap();

                rigid_body.position = mouse_pos - drag_offset;

            },
            None => {
                collider.position = mouse_pos - drag_offset;
            },
        }

        

        
    }

    fn update_is_dragging(&mut self, space: &mut Space) {

        if *self.get_dragging() {
            *self.color() = GREEN.into();
        }

        else {
            *self.color() = RED.into();
        }

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

        let mouse_pos = macroquad_to_rapier(
            &Vec2::new(mouse_position().0, mouse_position().1)
        );

        // if the body does not contain the mouse, but the button is down, we just dont do anything, because this is still a valid dragging state IF we are already dragging
        if !space.query_point(mouse_pos).contains(self.get_collider_handle()) {
            return
        }

        // at this point we know we will update dragging to true, but we want to check if this is a change from the last tick, so that we can set the mouse offset only when we begin dragging
        if !*self.get_dragging() {

            let collider = space.get_collider(self.get_collider_handle()).unwrap().clone();

            match collider.parent {

                Some(rigid_body_handle) => {
                    let rigid_body = space.get_rigid_body(&rigid_body_handle).unwrap();

                    *self.get_drag_offset() = Some(
                        Vec2::new(mouse_pos.x - rigid_body.position.x, mouse_pos.y - rigid_body.position.y)
                    );

                },
                None => {

                    let collider = space.get_collider(self.get_collider_handle()).unwrap();

                    *self.get_drag_offset() = Some(
                        Vec2::new(mouse_pos.x - collider.position.x, mouse_pos.y - collider.position.y)
                    );
                },
            }

            
        }

        *self.get_dragging() = true;

        

    }

    async fn draw(&mut self, camera_offset: &Vec2, space: &Space) {
        let collider_handle = self.get_collider_handle();
        let collider = space.get_collider(collider_handle).expect("Invalid collider handle");

        // if the collider has a rigid body, then we use it's position instead
        let (position, rotation) = match collider.parent.clone() {
            Some(rigid_body_handle) => {
                
                let rigid_body = space.get_rigid_body(&rigid_body_handle).unwrap();

                (rigid_body.position, rigid_body.rotation)
                

            },
            None => (collider.position, collider.rotation)
        };

        // draw the outline
        if *self.get_selected() {
            macroquad::shapes::draw_rectangle_ex(
                position.x, 
                ((position.y) * -1.) + screen_height(), 
                collider.hx * 2.5, 
                collider.hy * 2.5, 
                DrawRectangleParams { offset: macroquad::math::Vec2::new(0.5, 0.5), rotation: rotation * -1., color: WHITE }
            );
        } 

        macroquad::shapes::draw_rectangle_ex(
            position.x, 
            ((position.y) * -1.) + screen_height(), 
            collider.hx * 2., 
            collider.hy * 2., 
            DrawRectangleParams { offset: macroquad::math::Vec2::new(0.5, 0.5), rotation: rotation * -1., color: self.color().into() }
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

pub trait Texture: HasRect + Scale {
    async fn draw(&self, textures: &mut HashMap<String, Texture2D>, camera_offset: &Vec2) {
        
        // load texture if not already
        if !textures.contains_key(&self.get_texture_path()) {
            let texture = load_texture(&self.get_texture_path()).await.unwrap();
            
            texture.set_filter(texture::FilterMode::Nearest);

            textures.insert(self.get_texture_path(), texture);
        }

        let texture = textures.get(&self.get_texture_path()).unwrap();

        let scaled_texture_size = Vec2 {
            x: texture.width() * self.get_scale() as f32,
            y: texture.height() * self.get_scale() as f32
        };

        // macroquad::shapes::draw_rectangle(
        //     self.get_rect().x,
        //     self.get_rect().y,
        //     self.get_rect().w, 
        //     self.get_rect().h,
        //     color::RED
        // );

        macroquad::texture::draw_texture_ex(
            texture,
            self.get_rect().x + camera_offset.x,
            self.get_rect().y + camera_offset.y,
            WHITE,
            macroquad::texture::DrawTextureParams {
                dest_size: Some(scaled_texture_size.into()),
                ..Default::default()
            },
         );

    }

    fn get_texture_path(&self) -> String;

    fn set_texture_path(&mut self, texture_path: String);
}

pub trait HasOwner {
    fn get_owner(&self) -> String;

    fn set_owner(&mut self, uuid: String);
}

pub trait Scale {
    fn get_scale(&self) -> u32;
}

pub trait Draggable: HasRect + Velocity {
    fn drag(&mut self) {

        if input::is_mouse_button_down(input::MouseButton::Left) & self.get_rect().contains(Vec2{x: input::mouse_position().0, y: input::mouse_position().1}) {
            self.set_dragging(true)
        }

        if input::is_mouse_button_released(input::MouseButton::Left) {
            self.set_dragging(false)
        }

        if !self.get_dragging() {
            return;
        }

        let mouse_pos = Vec2{x: macroquad::input::mouse_position().0, y: macroquad::input::mouse_position().1};

        let rect = self.get_rect();

        let distance_to_mouse = Vec2::new(
            mouse_pos.x - rect.x,
            mouse_pos.y - rect.y
        );
        
        self.set_velocity(
            distance_to_mouse.normalize() * 1000.
        );

    }

    fn get_dragging(&self) -> bool;

    fn set_dragging(&mut self, dragging: bool);

}

pub trait Sound {
    async fn play_sound(&self, sounds: &mut HashMap<String, macroquad::audio::Sound>) {
        // load texture if not already
        if !sounds.contains_key(&self.get_sound_path()) {
            let sound = load_sound(&self.get_sound_path()).await.unwrap();

            sounds.insert(self.get_sound_path(), sound);
        }

        let sound = sounds.get(&self.get_sound_path()).unwrap();

        audio::play_sound_once(sound);
    }

    fn get_sound_path(&self) -> String;

    fn set_sound_path(&mut self, sound_path: String);

}

