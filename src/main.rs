use core::convert::{TryFrom, TryInto};
use std::error::Error;
use macroquad::prelude::*;

/// A very generic error type
type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// The divisor we use for fixed point conversion
const FIXED_POINT_DIVISOR: i16 = 40;

/// Width of the internal game field
const GAME_FIELD_WIDTH:  Fxpt = Fxpt(800 * FIXED_POINT_DIVISOR);

/// Height of the internal game field
const GAME_FIELD_HEIGHT: Fxpt = Fxpt(600 * FIXED_POINT_DIVISOR);

/// Player X coord
const PLAYER_X: Fxpt = Fxpt(20 * FIXED_POINT_DIVISOR);

/// Width and height dimension of the players collision square
const PLAYER_SIZE: Fxpt = Fxpt(40 * FIXED_POINT_DIVISOR);

/// A fixed point integer, converting to a float is done by dividing by
/// [`FIXED_POINT_DIVISOR`]
#[derive(Clone, Copy)]
struct Fxpt(i16);

impl TryFrom<i16> for Fxpt {
    type Error = Box<dyn Error>;

    fn try_from(val: i16) -> Result<Self> {
        val.checked_mul(FIXED_POINT_DIVISOR)
            .map(|x| Fxpt(x))
            .ok_or_else(|| "Integer overflow on i16 to Fxpt".into())
    }
}

impl TryFrom<Fxpt> for f32 {
    type Error = Box<dyn Error>;

    fn try_from(val: Fxpt) -> Result<Self> {
        let tmp = val.0 as f32 / FIXED_POINT_DIVISOR as f32;
        if tmp.is_finite() {
            Ok(tmp)
        } else {
            Err("Fixed-point conversion to f32 was not finite".into())
        }
    }
}

/// An object to render onto the screen
#[derive(Clone, Copy)]
enum Object {
    /// Draw a rectangle
    Rectangle { x: Fxpt, y: Fxpt, width: Fxpt, height: Fxpt, color: Color },

    /// Draw a polygon
    Polygon {
        x: Fxpt, y: Fxpt, sides: u8,
        radius: Fxpt, rotation: Fxpt, color: Color,
    },
}

/// The game field which is used for the deterministic game. All dimensions
/// and positions are based on fixed-point
struct GameField {
    /// Number of frames rendered (first frame during rendering will observe
    /// this as zero). Thus, this is incremented _after_ rendering is complete
    frames: u64,

    /// Player Y coord
    player_y: Fxpt,

    /// List of [`Object`]s to draw
    objects: Vec<Object>,
}

impl GameField {
    fn new() -> Self {
        Self {
            frames:   0,
            player_y: Fxpt(GAME_FIELD_HEIGHT.0 / 2),
            objects:  Vec::new(),
        }
    }

    /// Draw a player where ([`PLAYER_X`], `self.player_y`) is the top left
    /// coord of the players collision square which is [`PLAYER_SIZE`]
    fn draw_player(&mut self) {
        // Default player
        self.objects.push(Object::Rectangle {
            x:      PLAYER_X,
            y:      self.player_y,
            width:  PLAYER_SIZE,
            height: PLAYER_SIZE,
            color:  RED,
        });
    }

    fn render(&mut self) -> Result<()> {
        let offset_x = 10.;
        let offset_y = 50.;
        let target_w = screen_width()  - offset_x - 10.;
        let target_h = screen_height() - offset_y - 10.;
        let scale_x  = target_w / f32::try_from(GAME_FIELD_WIDTH)?;
        let scale_y  = target_h / f32::try_from(GAME_FIELD_HEIGHT)?;

        // Pick the smaller of the two scales and maintain aspect ratio
        let scale = scale_x.min(scale_y);

        // Recompute targets
        let target_w = scale * f32::try_from(GAME_FIELD_WIDTH)?;
        let target_h = scale * f32::try_from(GAME_FIELD_HEIGHT)?;

        // Clear all render objects
        self.objects.clear();
        
        // Add the player to the object list
        self.draw_player();
        
        // Clear the background
        clear_background(BLACK);

        // Draw the game field bounding box
        draw_rectangle_lines(offset_x, offset_y, target_w, target_h, 2., BLUE);

        // Render the objects
        for object in &self.objects {
            match object {
                &Object::Rectangle { x, y, width, height, color } => {
                    draw_rectangle(
                        f32::try_from(x)? * scale + offset_x,
                        f32::try_from(y)? * scale + offset_y,
                        f32::try_from(width)?  * scale,
                        f32::try_from(height)? * scale,
                        color);
                }
                &Object::Polygon { x, y, sides, radius, rotation, color } => {
                    draw_poly(
                        f32::try_from(x)? * scale + offset_x,
                        f32::try_from(y)? * scale + offset_y,
                        sides,
                        f32::try_from(radius)? * scale,
                        rotation.try_into()?,
                        color);
                }
            }
        }
        
        draw_text(&format!("FPS {}", get_fps()), 0., 16., 16., WHITE);

        // End of rendering
        self.frames += 1;
        Ok(())
    }
}

async fn game() -> Result<()> {
    let mut field = GameField::new();

    loop {
        field.render()?;
        next_frame().await;
    }
}

#[macroquad::main("BasicShapes")]
async fn main() {
    game().await.expect("Failed to run game");
}

