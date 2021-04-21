use macroquad::prelude::*;

/// Game field X coord
const GAME_X: f32 = 20.;

/// Game field Y coord
const GAME_Y: f32 = 60.;

/// Width of the game field
const GAME_WIDTH:  f32 = 800.;

/// Height of the game field
const GAME_HEIGHT: f32 = 600.;

/// Gravity
const GRAVITY: f32 = 1.25;

/// Friction
const FRICTION: f32 = 0.9;

/// The player
struct Player {
    x: f32,
    y: f32,

    width:  f32,
    height: f32,

    yspeed: f32,
}

#[macroquad::main("BasicShapes")]
async fn main() {
    let mut player = Player {
        x:      30.,
        y:      30.,
        width:  64.,
        height: 64.,
        yspeed: 0.,
    };

    for frame in 1u64.. {
        clear_background(BLACK);
        
        // Draw the game boundaries
        draw_rectangle_lines(GAME_X - 1., GAME_Y - 1.,
                             GAME_WIDTH + 2., GAME_HEIGHT + 2., 1., GRAY);

        if frame % 4 == 0 {
            // Fly if the player is clicking
            if is_mouse_button_down(MouseButton::Left) {
                player.yspeed -= 2.;
            }

            // Apply gravity and friction
            player.yspeed += GRAVITY;
            player.yspeed *= FRICTION;

            // Adjust the Y position of the player
            player.y = (player.y + player.yspeed)
                .clamp(0., GAME_HEIGHT - player.height);
        }

        // Draw the player
        draw_rectangle(GAME_X + player.x, GAME_Y + player.y,
                       player.width, player.height, GREEN);
        
        draw_text(&format!("FPS {:6} | Yspeed {:#018x}", get_fps(), player.yspeed.to_bits()), 0., 16., 16., WHITE);
        next_frame().await;
    }
}

