use diff::Diff;
use macroquad::{color::{Color, BLACK, WHITE}, input::{self, mouse_position}, math::{Rect, Vec2}, shapes::draw_rectangle_lines};
use serde::{Deserialize, Serialize};

use crate::{mouse_world_pos, rapier_mouse_world_pos};

#[derive(Serialize, Deserialize, Diff, PartialEq, Clone)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]
pub struct Menu {
    items: Vec<Button>,
    position: Vec2,
    pub color: Color,
    pub containing_rect: Rect
}

impl Menu {

    pub fn new(position: Vec2, color: Color) -> Self {
        Self {
            items: vec![],
            position: position,
            color: color,
            containing_rect: Rect::new(position.x, position.y, 0., 0.)
        }
    }

    pub fn update(&mut self, camera_rect: &Rect) {

        // reset containing rect because the menu items can change
        self.containing_rect = Rect::new(self.position.x, self.position.y, 0., 0.);

        for menu_item in &mut self.items {
            menu_item.update(camera_rect);

            self.containing_rect = self.containing_rect.combine_with(menu_item.rect);
        }

    }

    pub fn get_menu_items(&self) -> &Vec<Button> {
        &self.items
    }

    pub fn add_button(&mut self, text: String) {

        self.items.push(
            Button { 
                rect: Rect { 
                    x: self.position.x, 
                    y: self.position.y + (30. * self.items.len() as f32), 
                    w: 150., 
                    h: 30. 
                }, 
                text: text, 
                hovered: false, 
                clicked: false, 
                color: self.color
            }
        )
    }

    pub async fn draw(&self) {

        for item in &self.items {
            item.draw().await;
        }

        draw_rectangle_lines(self.containing_rect.x, self.containing_rect.y, self.containing_rect.w, self.containing_rect.h, 3., WHITE);

    }
}

#[derive(Serialize, Deserialize, Diff, PartialEq, Clone)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]
pub struct Button {
    pub rect: Rect,
    pub text: String,
    pub hovered: bool,
    pub clicked: bool,
    pub color: Color
}

impl Button {

    pub fn new(text: String, rect: Rect, color: macroquad::color::Color) -> Self {
        Self {
            rect,
            text,
            hovered: false,
            clicked: false,
            color,
        }
    }
    pub async fn draw(&self) {

        let (rect_color, font_color) = match self.hovered {
            true => (WHITE, BLACK),
            false => (self.color.into(), WHITE)
        };

        
        macroquad::shapes::draw_rectangle(self.rect.x, self.rect.y, self.rect.w, self.rect.h, rect_color);
        macroquad::shapes::draw_rectangle_lines(self.rect.x, self.rect.y, self.rect.w, self.rect.h, 3., BLACK);
        macroquad::text::draw_text(&self.text, self.rect.x + 3., self.rect.y + self.rect.h / 2., 20., font_color);
    }

    pub fn update(&mut self, camera_rect: &Rect) {

        let mouse_position = Vec2::from_array(mouse_position().into());

        self.hovered = false;
        self.clicked = false;

        if self.rect.contains(
            Vec2::new(mouse_position.x, mouse_position.y)
        ) {

            self.hovered = true;

            if input::is_mouse_button_pressed(input::MouseButton::Left) {
                self.clicked = true;
            }
        }
    }
}


