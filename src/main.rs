use std::error::Error;
use std::collections::VecDeque;
use macroquad::prelude::*;

/// A very generic error type
type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Enables scaling of the internal game field to the output screen size
const SCALE_OUTPUT: bool = true;

/// The divisor we use for fixed point conversion
const FIXED_POINT_SHIFT:   u32 = 5;
const FIXED_POINT_DIVISOR: i16 = 1 << FIXED_POINT_SHIFT;

/// Width of the internal game field
const GAME_FIELD_WIDTH:  Fxpt = Fxpt(400 * FIXED_POINT_DIVISOR);

/// Height of the internal game field
const GAME_FIELD_HEIGHT: Fxpt = Fxpt(300 * FIXED_POINT_DIVISOR);

/// Player X coord
const PLAYER_X: Fxpt = Fxpt(100 * FIXED_POINT_DIVISOR);

/// Width and height dimension of the players collision square
const PLAYER_SIZE: Fxpt = Fxpt(48 * FIXED_POINT_DIVISOR);

/// The width of a wall or obstacle
const OBSTACLE_WIDTH: Fxpt = Fxpt(25 * FIXED_POINT_DIVISOR);

/// Gravity the player experiences
const GRAVITY: Fxpt = Fxpt((1.6 * FIXED_POINT_DIVISOR as f32) as i16);

/// Friction the player experiences
const FRICTION: Fxpt = Fxpt((0.9 * FIXED_POINT_DIVISOR as f32) as i16);

/// Speed change upon input on each frame
const INPUT_IMPULSE: Fxpt = Fxpt(2 * FIXED_POINT_DIVISOR);

/// A fixed point integer, converting to a float is done by dividing by
/// [`FIXED_POINT_DIVISOR`]
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
struct Fxpt(i16);

impl From<i16> for Fxpt {
    fn from(val: i16) -> Self {
        Fxpt(val * FIXED_POINT_DIVISOR)
    }
}

impl From<Fxpt> for f32 {
    fn from(val: Fxpt) -> Self {
        let tmp = val.0 as f32 / FIXED_POINT_DIVISOR as f32;
        if tmp.is_finite() {
            tmp
        } else {
            panic!("Fixed-point conversion to f32 was not finite");
        }
    }
}

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Self(0x1337133713371337)
    }

    fn rand(&mut self) -> u64 {
        let ret = self.0;
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 43;
        ret
    }
}

/// An object to render onto the screen
#[derive(Clone, Copy)]
#[allow(unused)]
enum Object {
    /// Draw a rectangle
    Rectangle { x: Fxpt, y: Fxpt, width: Fxpt, height: Fxpt, color: Color },

    /// Draw a polygon
    Polygon {
        x: Fxpt, y: Fxpt, sides: u8,
        radius: Fxpt, rotation: Fxpt, color: Color,
    },
}

#[derive(Clone, Copy)]
struct Obstacle {
    x:      Fxpt,
    y:      Fxpt,
    width:  Fxpt,
    height: Fxpt,
}

/// The game field which is used for the deterministic game. All dimensions
/// and positions are based on fixed-point
struct GameField {
    /// Random number generator for the game
    rng: Rng,

    /// Number of frames rendered (first frame during rendering will observe
    /// this as zero). Thus, this is incremented _after_ rendering is complete
    frames: u64,

    /// Number of physics frames
    physics_frames: u64,

    /// Player Y coord
    player_y: Fxpt,

    /// Player speed
    player_speed: Fxpt,

    /// Time (in seconds) of the last frame
    last_frame: f64,

    /// Start time (in seconds) when the [`GameField`] was created
    start_time: f64,

    /// List of [`Object`]s to draw
    objects: Vec<Object>,

    walls: Vec<Obstacle>,
    obstacles: Vec<Obstacle>,

    wall_skew: Fxpt,

    /// Physics frame of the last generated obstacle
    last_obstacle: u64,

    /// Tracks if we lost
    dead: bool,

    /// Tracks if we should replay the `inputs` rather than use interactive
    /// inputs
    replay: Option<VecDeque<u8>>,

    /// Tracks the mouse input state each physics frame
    inputs: VecDeque<u8>,
}

impl GameField {
    fn new() -> Self {
        Self {
            rng:            Rng::new(),
            frames:         0,
            physics_frames: 0,
            player_y:       Fxpt(GAME_FIELD_HEIGHT.0 / 2),
            objects:        Vec::new(),
            player_speed:   Fxpt(0),
            last_frame:     0.,
            start_time:     get_time(),
            walls:          Vec::new(),
            obstacles:      Vec::new(),
            last_obstacle:  0,
            wall_skew:      Fxpt(0),
            dead:           false,
            replay:         None,
            inputs:         VecDeque::new(),
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
            color:  Color::from_rgba(
                (self.physics_frames as u8).wrapping_mul(3),
                (self.physics_frames as u8).wrapping_mul(7),
                (self.physics_frames as u8).wrapping_mul(5), 0xff),
        });
    }

    /// A color generator from Desu_Used
    fn pastel_rainbow(x: f32) -> (u8, u8, u8) {
        const TAU: f32 = core::f32::consts::PI * 2.0;
        let r = (x * TAU + 0.274).sin() * 40.0 + 213.0;
        let g = (x * TAU - 1.616).sin() * 40.0 + 213.0;
        let b = (x * TAU - 3.918).sin() * 46.0 + 207.0;
        (r as u8, g as u8, b as u8)
    }

    fn render(&mut self) -> Result<bool> {
        let offset_x = 10.;
        let offset_y = 50.;
        let (target_w, target_h) = if SCALE_OUTPUT {
            (screen_width() - offset_x - 10., screen_height() - offset_y - 10.)
        } else {
            (f32::from(GAME_FIELD_WIDTH), f32::from(GAME_FIELD_HEIGHT))
        };
        let scale_x  = target_w / f32::from(GAME_FIELD_WIDTH);
        let scale_y  = target_h / f32::from(GAME_FIELD_HEIGHT);

        // Pick the smaller of the two scales and maintain aspect ratio
        let scale = scale_x.min(scale_y);

        // Recompute targets
        let target_w = scale * f32::from(GAME_FIELD_WIDTH);
        let target_h = scale * f32::from(GAME_FIELD_HEIGHT);
            
        if self.dead && is_key_pressed(KeyCode::Space) {
            return Ok(true);
        }

        let time = get_time();
        if !self.dead && time - self.last_frame >= 1. / 60. {
            // Update player speed if we're flying
            if (self.replay.is_none() &&
                    is_mouse_button_down(MouseButton::Left)) ||
                    self.replay.as_mut()
                        .and_then(|x| x.pop_front()) == Some(b'1') {
                self.player_speed =
                    Fxpt(self.player_speed.0 - INPUT_IMPULSE.0);
                self.inputs.push_back(b'1');
            } else {
                self.inputs.push_back(b'0');
            }
            
            // Move the map (both walls and obstacles)
            for obstacle in self.walls.iter_mut()
                    .chain(self.obstacles.iter_mut()) {
                obstacle.x = Fxpt(obstacle.x.0 - Fxpt::from(8).0);
            }

            // Create walls
            let last_x = self.walls.get(
                self.walls.len().wrapping_sub(1))
                .map(|x| x.x)
                .unwrap_or(Fxpt(GAME_FIELD_WIDTH.0 - OBSTACLE_WIDTH.0));
            if last_x <= Fxpt(GAME_FIELD_WIDTH.0 - OBSTACLE_WIDTH.0) {
                // Compute the gap to use between the walls
                // We start at a 250 pixel gap, descend to a 180 pixel gap
                // at a rate of one pixel per second, which is approx 70
                // seconds until minimum size.
                let gap_reduction = (self.physics_frames / 32).min(70) as i16;
                let gap = Fxpt::from(250 - gap_reduction);

                let wall_size = Fxpt((GAME_FIELD_HEIGHT.0 - gap.0) / 2);

                self.wall_skew = Fxpt((self.wall_skew.0 +
                    self.rng.rand() as i16 % (FIXED_POINT_DIVISOR * 8))
                    .clamp(-wall_size.0, wall_size.0));

                self.walls.push(Obstacle {
                    x:      Fxpt(last_x.0 + OBSTACLE_WIDTH.0),
                    y:      Fxpt(0),
                    width:  OBSTACLE_WIDTH,
                    height: Fxpt(wall_size.0 + self.wall_skew.0),
                });
                
                self.walls.push(Obstacle {
                    x:      Fxpt(last_x.0 + OBSTACLE_WIDTH.0),
                    y:      Fxpt(GAME_FIELD_HEIGHT.0 - (wall_size.0 -
                                 self.wall_skew.0)),
                    width:  OBSTACLE_WIDTH,
                    height: Fxpt(wall_size.0 - self.wall_skew.0),
                });

                if self.physics_frames - self.last_obstacle >= 30 {
                    let location = ((self.rng.rand() as u16) %
                        (gap.0 - Fxpt::from(60).0) as u16) as i16;

                    self.obstacles.push(Obstacle {
                        x:      Fxpt(last_x.0 + OBSTACLE_WIDTH.0),
                        y:      Fxpt(wall_size.0 + self.wall_skew.0 +
                                     location),
                        width:  OBSTACLE_WIDTH,
                        height: Fxpt::from(60),
                    });

                    self.last_obstacle = self.physics_frames;
                }
            }

            // Cull walls and obstacles which are off screen
            self.walls.retain(|x| {
                Fxpt(x.x.0 + x.width.0) > Fxpt(0)
            });
            self.obstacles.retain(|x| {
                Fxpt(x.x.0 + x.width.0) > Fxpt(0)
            });

            // Apply physics
            self.player_speed = Fxpt(self.player_speed.0 + GRAVITY.0);
            self.player_speed =
                Fxpt((self.player_speed.0 >> FIXED_POINT_SHIFT) * FRICTION.0);

            // Adjust player position
            self.player_y = Fxpt(self.player_y.0 + self.player_speed.0);

            // Bound player
            self.player_y = Fxpt(
                self.player_y.0.clamp(0, GAME_FIELD_HEIGHT.0 - PLAYER_SIZE.0));

            // Check collisions
            for obstacle in self.obstacles.iter().chain(self.walls.iter()) {
                let a1 = obstacle.x.0;
                let a2 = obstacle.x.0 + obstacle.width.0;
                let b1 = PLAYER_X.0;
                let b2 = PLAYER_X.0 + PLAYER_SIZE.0;
                
                let c1 = obstacle.y.0;
                let c2 = obstacle.y.0 + obstacle.height.0;
                let d1 = self.player_y.0;
                let d2 = self.player_y.0 + PLAYER_SIZE.0;

                if a1.max(b1) < a2.min(b2) && c1.max(d1) < c2.min(d2) {
                    self.dead = true;
                }
            }

            // Update the last frame time
            self.last_frame = time;

            // Update physics frames
            self.physics_frames += 1;
        }

        // Clear all render objects
        self.objects.clear();

        // Draw obstacles
        for &obstacle in self.obstacles.iter().chain(self.walls.iter()) {
            // Recompute the start and end to make sure we don't render outside
            // the game window
            let x = obstacle.x.0.max(0);
            let end =
                (obstacle.x.0 + obstacle.width.0).min(GAME_FIELD_WIDTH.0);

            let (r, g, b) = Self::pastel_rainbow(
                f32::from(obstacle.x) * 0.003);

            self.objects.push(Object::Rectangle {
                x:      Fxpt(x),
                y:      obstacle.y,
                width:  Fxpt(end - x),
                height: obstacle.height,
                color:  Color::from_rgba(r, g, b, 0xff),
            });
        }
        
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
                        f32::from(x) * scale + offset_x,
                        f32::from(y) * scale + offset_y,
                        f32::from(width)  * scale,
                        f32::from(height) * scale,
                        color);
                }
                &Object::Polygon { x, y, sides, radius, rotation, color } => {
                    draw_poly(
                        f32::from(x) * scale + offset_x,
                        f32::from(y) * scale + offset_y,
                        sides,
                        f32::from(radius) * scale,
                        rotation.into(),
                        color);
                }
            }
        }

        // End of rendering
        self.frames += 1;
        Ok(false)
    }
}

async fn game() -> Result<()> {
    // Run the replay file if there is an arg
    let replay: Option<VecDeque<u8>> = std::env::args().nth(1).map(|x| {
        std::fs::read(x).expect("Failed to load replay input").into()
    });

    let mut high_score = 0u64;

    'restart: loop {
        let mut field = GameField::new();
        field.replay = replay.clone();

        #[cfg(not(target_arch = "wasm32"))]
        let mut new_score = false;

        loop {
            if field.render()? {
                #[cfg(not(target_arch = "wasm32"))]
                if new_score {
                    std::fs::write("inputs.bin",
                        field.inputs.iter().copied().collect::<Vec<_>>())?;
                }
                continue 'restart;
            }
       
            if field.physics_frames > high_score {
                #[cfg(not(target_arch = "wasm32"))]
                { new_score = true; }

                high_score = field.physics_frames;
            }

            draw_text(&format!("Average FPS {:9.3} | Score {:10} | \
                                High score {:10} | {:10.3}",
                field.frames as f64 / (get_time() - field.start_time),
                field.physics_frames, high_score, field.player_speed.0),
                0., 20., 32., WHITE);

            next_frame().await;
        }
    }
}

#[macroquad::main("BasicShapes")]
async fn main() {
    game().await.expect("Failed to run game");
}

