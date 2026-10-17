use bevy::prelude::*;
use crate::ball::{Ball, BallKickCooldown};
use crate::game::{GameState, Team, Velocity};
use crate::player::{Controlled, Player};

// ── Plugin ──────────────────────────────────────────────────────────

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (handle_movement, handle_kick, handle_pass)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

// ── Systems ─────────────────────────────────────────────────────────

/// WASD / Arrow keys → move the controlled player.
fn handle_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&Player, &mut Velocity), With<Controlled>>,
) {
    let Ok((player, mut vel)) = query.get_single_mut() else {
        return;
    };

    let mut dir = Vec3::ZERO;

    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        dir.z -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        dir.z += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        dir.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        dir.x += 1.0;
    }

    // Sprint with Shift
    let sprint = if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
        1.45
    } else {
        1.0
    };

    vel.0 = if dir.length_squared() > 0.0 {
        dir.normalize() * player.speed * sprint
    } else {
        Vec3::ZERO
    };
}

/// Space → kick / shoot the ball with power (even while sprinting/dribbling).
fn handle_kick(
    keyboard: Res<ButtonInput<KeyCode>>,
    player_q: Query<(&Transform, &Velocity), (With<Controlled>, With<Player>, Without<Ball>)>,
    mut ball_q: Query<(&Transform, &mut Velocity, &mut BallKickCooldown), With<Ball>>,
) {
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }

    let Ok((p_tf, p_vel)) = player_q.get_single() else {
        return;
    };
    let Ok((b_tf, mut b_vel, mut kick_cd)) = ball_q.get_single_mut() else {
        return;
    };

    // Must be close enough to kick
    let dist = p_tf.translation.distance(b_tf.translation);
    if dist > 3.0 {
        return;
    }

    // Kick in player's movement direction, or toward opponent goal (+X) if stationary
    let kick_dir = if p_vel.0.length_squared() > 0.25 {
        p_vel.0.normalize()
    } else {
        Vec3::X
    };

    // Launch shot and unlock ball from dribble magnet so it rockets away cleanly
    b_vel.0 = kick_dir * 38.0;
    kick_cd.0 = 0.35;
}

/// X / K / J → Ground pass to teammate in aim direction (even while sprinting/dribbling)
fn handle_pass(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    player_q: Query<(Entity, &Transform, &Velocity), (With<Controlled>, With<Player>, Without<Ball>)>,
    teammates_q: Query<(Entity, &Transform, &Player), Without<Ball>>,
    mut ball_q: Query<(&Transform, &mut Velocity, &mut BallKickCooldown), With<Ball>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyX)
        && !keyboard.just_pressed(KeyCode::KeyK)
        && !keyboard.just_pressed(KeyCode::KeyJ)
    {
        return;
    }

    let Ok((passer_entity, p_tf, p_vel)) = player_q.get_single() else {
        return;
    };
    let Ok((b_tf, mut b_vel, mut kick_cd)) = ball_q.get_single_mut() else {
        return;
    };

    // Must be near the ball to pass
    let dist_to_ball = p_tf.translation.distance(b_tf.translation);
    if dist_to_ball > 3.0 {
        return;
    }

    // Aim direction: either player's movement direction, or facing toward opponent goal (+X)
    let aim_dir = if p_vel.0.length_squared() > 0.25 {
        p_vel.0.normalize()
    } else {
        Vec3::X
    };

    // Find best teammate (Home team):
    let mut best_target: Option<(Entity, Vec3, f32)> = None;
    let mut highest_score = -999.0;

    for (entity, t_tf, player) in &teammates_q {
        if entity == passer_entity || player.team != Team::Home {
            continue;
        }

        let to_teammate = t_tf.translation - p_tf.translation;
        let dist = to_teammate.length();

        // Reasonable pass distance limits
        if dist < 2.0 || dist > 55.0 {
            continue;
        }

        let dir_to_teammate = to_teammate.normalize();
        let dot = dir_to_teammate.dot(aim_dir);

        // Teammates in the forward cone of aim
        if dot > -0.3 {
            let score = dot * 3.0 - (dist / 45.0);
            if score > highest_score {
                highest_score = score;
                best_target = Some((entity, t_tf.translation, dist));
            }
        }
    }

    if let Some((receiver_entity, target_pos, dist)) = best_target {
        // Direct the ball with controlled, natural ground pass strength
        let pass_dir = (target_pos - b_tf.translation).normalize();
        let speed = (dist * 0.95 + 7.5).clamp(10.0, 19.0);
        b_vel.0 = pass_dir * speed;
        kick_cd.0 = 0.35; // Unlock ball from dribble magnet so pass glides away cleanly

        // Switch control to the receiver so player can seamlessly receive the pass
        commands.entity(passer_entity).remove::<Controlled>();
        commands.entity(receiver_entity).insert(Controlled);
    } else {
        // Free ground pass in aim direction if no teammate is in cone
        b_vel.0 = aim_dir * 12.5;
        kick_cd.0 = 0.35;
    }
}
