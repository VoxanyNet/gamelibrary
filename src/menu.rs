use diff::Diff;
use futures::executor::block_on;
use lz4_flex::block;
use macroquad::{color::{Color, BLACK, WHITE}, input::{self, mouse_position}, math::{Rect, Vec2}, shapes::draw_rectangle_lines, text::{load_ttf_font, Font, TextParams}};
use nalgebra::OPoint;
use serde::{ser::SerializeStruct, Deserialize, Serialize};


#[derive(Serialize, Deserialize, Diff, PartialEq, Clone)]
#[diff(attr(
    #[derive(Serialize, Deserialize)]
))]
pub struct Menu {
    items: Vec<Button>,
    position: Vec2,
    pub color: Color,
    pub containing_rect: Rect,
    font_path: String,
    hovered_color: Color,
    hovered_text_color: Color,
}

impl Menu {

    pub fn new(position: Vec2, color: Color, font_path: String, hovered_color: Option<Color>, hovered_text_color: Option<Color>) -> Self {

        let hovered_color = match hovered_color {
            Some(hovered_color) => hovered_color,
            None => WHITE,
        };

        let hovered_text_color = match hovered_text_color {
            Some(hovered_color) => hovered_color,
            None => BLACK,
        };


        Self {
            items: vec![],
            position: position,
            color: color,
            containing_rect: Rect::new(position.x, position.y, 0., 0.),
            hovered_text_color,
            font_path,
            hovered_color
        }
    }

    pub fn update(&mut self, camera_rect: Option<&Rect>) {

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

    pub async fn add_button(&mut self, text: String) {

        self.items.push(
            Button { 
                rect: Rect { 
                    x: self.position.x, 
                    y: self.position.y + (30. * self.items.len() as f32), 
                    w: 150., 
                    h: 30.
                }, 
                text: text, 
                hovered_text_color: self.hovered_text_color,
                hovered: false, 
                clicked: false, 
                color: self.color,
                font_size: 20,
                font_path: self.font_path.clone(),
                font: load_ttf_font(&self.font_path).await.unwrap(),
                hovered_color: self.hovered_color
            
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

#[derive(Serialize, Clone)]
pub struct Button {
    pub rect: Rect,
    pub text: String,
    pub hovered: bool,
    pub clicked: bool,
    pub hovered_color: Color,
    pub hovered_text_color: Color,
    pub color: Color,
    pub font_size: u16,
    #[serde(skip)]
    pub font: Font,
    pub font_path: String
}

impl PartialEq for Button {
    fn eq(&self, other: &Self) -> bool {
        self.rect == other.rect && self.text == other.text && self.hovered == other.hovered && self.clicked == other.clicked && self.color == other.color && self.font_size == other.font_size && self.font_path == other.font_path
    }
}

impl <'de> Deserialize<'de> for Button {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where 
        D: serde::Deserializer<'de> 
    {
        #[derive(Deserialize)]
        struct ButtonHelper {
            pub rect: Rect,
            pub text: String,
            pub hovered: bool,
            pub clicked: bool,
            pub hovered_color: Color,
            pub hovered_text_color: Color,
            pub color: Color,
            pub font_size: u16,
            pub font_path: String
        }

        let helper = ButtonHelper::deserialize(deserializer)?;

        Ok(
            Button {
                rect: helper.rect,
                text: helper.text,
                hovered: helper.hovered,
                clicked: helper.clicked,
                hovered_color: helper.hovered_color,
                hovered_text_color: helper.hovered_text_color,
                color: helper.color,
                font_size: helper.font_size,
                font: Font::default(), // this will need to be fixed
                font_path: helper.font_path,
            }
        )
    }
}


#[derive(Serialize, Deserialize)]
pub struct ButtonDiff {
    rect: Option<Rect>,
    text: Option<String>,
    hovered: Option<bool>,
    clicked: Option<bool>,
    color: Option<Color>,
    font_size: Option<u16>,
    font_path: Option<String>,
    hovered_color: Option<Color>,
    hovered_text_color: Option<Color>
}

impl Diff for Button {
    type Repr = ButtonDiff;

    fn diff(&self, other: &Self) -> Self::Repr {
        let mut diff = ButtonDiff {
            rect: None,
            text: None,
            hovered: None,
            clicked: None,
            color: None,
            font_size: None,
            font_path: None,
            hovered_color: None,
            hovered_text_color: None
            
        };

        if self.rect != other.rect {
            diff.rect = Some(other.rect);
        }

        if self.hovered_text_color != other.hovered_text_color {
            diff.hovered_text_color = Some(other.hovered_text_color);
        }

        if self.text != other.text {
            diff.text = Some(other.text.clone());
        }

        if self.hovered_color != other.hovered_color {
            diff.hovered_color = Some(other.hovered_color);
        }

        if self.hovered != other.hovered {
            diff.hovered = Some(other.hovered);
        }

        if self.clicked != other.clicked {
            diff.clicked = Some(other.clicked);
        }

        if self.color != other.color {
            diff.color = Some(other.color);
        }

        if self.font_size != other.font_size {
            diff.font_size = Some(other.font_size);
        }

        if other.font_path != other.font_path {
            diff.font_path = Some(other.font_path.clone());
        };

        diff
    }

    fn apply(&mut self, diff: &Self::Repr) {
        if let Some(rect) = diff.rect {
            self.rect = rect;
        }

        if let Some(hovered_text_color) = diff.hovered_text_color {
            self.hovered_text_color = hovered_text_color.clone()
        }

        if let Some(text) = &diff.text {
            self.text = text.clone();
        }

        if let Some(hovered) = diff.hovered {
            self.hovered = hovered;
        }

        if let Some(clicked) = diff.clicked {
            self.clicked = clicked;
        }

        if let Some(hovered_color) = diff.hovered_color {
            self.hovered_color = hovered_color;
        }
        if let Some(color) = diff.color {
            self.color = color;
        }

        if let Some(font_size) = diff.font_size {
            self.font_size = font_size;
        }

        if let Some(font_path) = &diff.font_path {
            self.font_path = font_path.clone();
            self.font = Font::default() // this needs to be fixed
        }
    }

    fn identity() -> Self {
        Button {
            rect: Rect::identity(),
            text: String::identity(),
            hovered: bool::identity(),
            clicked: bool::identity(),
            color: Color::identity(),
            hovered_color: Color::identity(),
            font_size: u16::identity(),
            font: Font::default(),
            font_path: String::default(),
            hovered_text_color: Color::identity()
        }
    }
}


impl Button {

    pub async fn new(
        text: String, 
        rect: Rect, 
        color: macroquad::color::Color, 
        hovered_color: Option<Color>,
        hovered_text_color: Option<Color>,
        font_size: u16, 
        font_path: String
    ) -> Self {

        let hovered_color = match hovered_color {
            Some(hovered_color) => hovered_color,
            None => WHITE,
        };

        let hovered_text_color = match hovered_text_color {
            Some(hovered_text_color) => hovered_text_color,
            None => BLACK,
        };

        Self {
            rect,
            text,
            hovered: false,
            clicked: false,
            hovered_color,
            hovered_text_color,
            color,
            font_size,
            font: load_ttf_font(&font_path).await.unwrap(),
            font_path: font_path
        }
    }
    pub async fn draw(&self) {

        let (rect_color, font_color) = match self.hovered {
            true => (self.hovered_color, self.hovered_text_color),
            false => (self.color.into(), WHITE)
        };

        let mut font_params = TextParams::default();

        font_params.font = Some(&self.font);
        font_params.font_size = self.font_size;
        font_params.color = font_color;

        
        macroquad::shapes::draw_rectangle(self.rect.x, self.rect.y, self.rect.w, self.rect.h, rect_color);
        macroquad::shapes::draw_rectangle_lines(self.rect.x, self.rect.y, self.rect.w, self.rect.h, 3., BLACK);
        macroquad::text::draw_text_ex(&self.text, self.rect.x + 3., self.rect.y + self.rect.h / 2., font_params);
    }

    pub fn update(&mut self, _camera_rect: Option<&Rect>) {

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


