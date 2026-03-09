use crate::game::types::{Vec3, ViewAngles};
use crate::player::types::MovementState;

/// Input flags for a single movement tick
#[derive(Debug, Clone, Default)]
pub struct MoveInput {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub crouch: bool,
    pub walk: bool,
    pub sneak: bool,
    pub view_delta_pitch: f32,
    pub view_delta_yaw: f32,
}

/// Simulate one frame of player movement
///
/// Returns the new position and view angles.
pub fn simulate_movement(
    position: Vec3,
    view: ViewAngles,
    movement_state: &mut MovementState,
    input: &MoveInput,
    delta_secs: f32,
    base_speed: f32,
    is_on_ground: bool,
) -> (Vec3, ViewAngles) {
    // Update movement state
    let new_state = determine_movement_state(input, is_on_ground);
    *movement_state = new_state;

    let speed = base_speed * new_state.speed_multiplier();

    // Convert yaw to direction vectors
    let yaw_rad = view.yaw.to_radians();
    let forward_x = yaw_rad.cos();
    let forward_y = yaw_rad.sin();
    let right_x = (yaw_rad + std::f32::consts::FRAC_PI_2).cos();
    let right_y = (yaw_rad + std::f32::consts::FRAC_PI_2).sin();

    let mut wish_x = 0.0f32;
    let mut wish_y = 0.0f32;

    if input.forward {
        wish_x += forward_x;
        wish_y += forward_y;
    }
    if input.backward {
        wish_x -= forward_x;
        wish_y -= forward_y;
    }
    if input.right {
        wish_x += right_x;
        wish_y += right_y;
    }
    if input.left {
        wish_x -= right_x;
        wish_y -= right_y;
    }

    // Normalize wish direction
    let wish_len = (wish_x * wish_x + wish_y * wish_y).sqrt();
    if wish_len > 0.001 {
        wish_x /= wish_len;
        wish_y /= wish_len;
    }

    let new_x = position.x + wish_x * speed * delta_secs;
    let new_y = position.y + wish_y * speed * delta_secs;

    // Simple jump (no gravity simulation - game server handles vertical physics)
    let new_z = if input.jump && is_on_ground { position.z + 50.0 } else { position.z };

    // Update view angles
    let new_pitch = (view.pitch + input.view_delta_pitch).clamp(-89.0, 89.0);
    let new_yaw = (view.yaw + input.view_delta_yaw) % 360.0;

    (
        Vec3::new(new_x, new_y, new_z),
        ViewAngles::new(new_pitch, new_yaw),
    )
}

fn determine_movement_state(input: &MoveInput, is_on_ground: bool) -> MovementState {
    if !is_on_ground {
        return MovementState::InAir;
    }
    if input.crouch {
        return MovementState::Crouching;
    }
    let moving = input.forward || input.backward || input.left || input.right;
    if !moving {
        return MovementState::Standing;
    }
    if input.sneak {
        return MovementState::Sneaking;
    }
    if input.walk {
        return MovementState::Walking;
    }
    MovementState::Running
}
