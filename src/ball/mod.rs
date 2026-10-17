use bevy::prelude::*;
use crate::game::{GameState, Score, SetPieceEvent, Velocity};
use crate::pitch::{GOAL_DEPTH, GOAL_HEIGHT, GOAL_WIDTH, HALF_LEN, HALF_WID};
use crate::player::{Controlled, Player};

// ── Plugin ──────────────────────────────────────────────────────────

pub struct BallPlugin;

impl Plugin for BallPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GoalCooldown(0.0))
            .insert_resource(AwayGoalKickTimer::default())
            .add_systems(Startup, spawn_ball)
            .add_systems(
                Update,
                (
                    ball_movement,
                    ball_player_interaction,
                    detect_goal,
                    handle_away_goal_kick,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

// ── Components & resources ──────────────────────────────────────────

#[derive(Component, Default)]
pub struct Ball {
    pub is_out_of_bounds: bool,
}

/// Cooldown timer set when kicking or passing to prevent the dribble magnet from immediately cancelling shots/passes
#[derive(Component, Default)]
pub struct BallKickCooldown(pub f32);

#[derive(Resource)]
pub struct GoalCooldown(pub f32);

/// Auto-clear timer for opponent goalkeeper goal kicks
#[derive(Resource, Default)]
pub struct AwayGoalKickTimer(pub Option<f32>);

pub const BALL_RADIUS: f32 = 0.22;
const FRICTION: f32 = 0.988;

// ── Startup ─────────────────────────────────────────────────────────

fn spawn_ball(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Sphere::new(BALL_RADIUS)),
            material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.97, 0.97, 0.97),
                perceptual_roughness: 0.35,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, BALL_RADIUS, 0.0),
            ..default()
        },
        Ball::default(),
        Velocity::default(),
        BallKickCooldown(0.0),
    ));
}

// ── Systems ─────────────────────────────────────────────────────────

/// Move the ball, apply friction, update kick cooldown, and detect out-of-bounds for set-pieces.
fn ball_movement(
    time: Res<Time>,
    mut events: EventWriter<SetPieceEvent>,
    mut query: Query<(&mut Transform, &mut Velocity, &mut BallKickCooldown, &mut Ball)>,
) {
    let Ok((mut tf, mut vel, mut kick_cd, mut ball)) = query.get_single_mut() else {
        return;
    };
    let dt = time.delta_seconds();

    // Tick down kick/pass lockout timer
    if kick_cd.0 > 0.0 {
        kick_cd.0 = (kick_cd.0 - dt).max(0.0);
    }

    // While awaiting set piece (throw-in, corner, goal kick), ball rests still
    if ball.is_out_of_bounds {
        vel.0 = Vec3::ZERO;
        return;
    }

    // Integrate velocity
    tf.translation += vel.0 * dt;

    // Rolling friction
    vel.0 *= FRICTION;
    if vel.0.length() < 0.04 {
        vel.0 = Vec3::ZERO;
    }

    // Keep on the turf
    tf.translation.y = BALL_RADIUS;

    let in_goal_mouth = tf.translation.z.abs() < (GOAL_WIDTH * 0.5) - 0.1 && tf.translation.y < GOAL_HEIGHT;

    // ── Sideline boundaries (Throw-ins) ─────────────────────────────
    if tf.translation.z.abs() > HALF_WID {
        if !ball.is_out_of_bounds {
            ball.is_out_of_bounds = true;
            let out_x = tf.translation.x.clamp(-HALF_LEN + 3.0, HALF_LEN - 3.0);
            let z_sign = tf.translation.z.signum();
            events.send(SetPieceEvent::ThrowIn { x: out_x, z_sign });
        }
        vel.0 = Vec3::ZERO;
        return;
    }

    // ── Endlines (Goal Kicks, Corners, or Goals) ────────────────────
    if tf.translation.x.abs() > HALF_LEN {
        if !in_goal_mouth {
            if !ball.is_out_of_bounds {
                ball.is_out_of_bounds = true;
                let x_sign = tf.translation.x.signum();
                let z_sign = tf.translation.z.signum();

                // Near corner flag (> 17.0m from center) = Corner Kick; inside = Goal Kick
                if tf.translation.z.abs() > 17.0 {
                    events.send(SetPieceEvent::CornerKick { x_sign, z_sign });
                } else {
                    events.send(SetPieceEvent::GoalKick { x_sign });
                }
            }
            vel.0 = Vec3::ZERO;
        } else {
            // Inside the goal net! Softly collide with the back of the net
            let net_back = HALF_LEN + GOAL_DEPTH - 0.25;
            if tf.translation.x.abs() > net_back {
                tf.translation.x = tf.translation.x.signum() * net_back;
                vel.0.x *= -0.15; // Net absorbs energy
                vel.0.z *= 0.3;
            }

            // Side netting collision inside goal
            let net_side = (GOAL_WIDTH * 0.5) - 0.15;
            if tf.translation.z.abs() > net_side {
                tf.translation.z = tf.translation.z.signum() * net_side;
                vel.0.z *= -0.2;
            }
        }
    }
}

/// Responsive PES dribbling with smooth orbital turning around the player's body
fn ball_player_interaction(
    player_q: Query<(&Transform, &Velocity, Option<&Controlled>), (With<Player>, Without<Ball>)>,
    mut ball_q: Query<(&mut Transform, &mut Velocity, &BallKickCooldown, &Ball), With<Ball>>,
    goal_cd: Res<GoalCooldown>,
) {
    let Ok((mut ball_tf, mut ball_vel, kick_cd, ball)) = ball_q.get_single_mut() else {
        return;
    };

    // Do not interfere with ball during set pieces or goal celebrations
    if ball.is_out_of_bounds || goal_cd.0 > 0.0 {
        return;
    }

    let in_kick_cooldown = kick_cd.0 > 0.0;

    for (p_tf, p_vel, controlled) in &player_q {
        let dx = ball_tf.translation.x - p_tf.translation.x;
        let dz = ball_tf.translation.z - p_tf.translation.z;
        let dist = (dx * dx + dz * dz).sqrt();

        if controlled.is_some() {
            // If ball was just kicked or passed, DO NOT pull it back into the dribble pocket!
            if in_kick_cooldown {
                continue;
            }

            // ── Active Player: Smooth PES Dribbling & Turning ────────
            let control_radius = 2.3;
            if dist < control_radius {
                let speed = p_vel.0.length();

                if speed > 0.5 {
                    let move_dir = p_vel.0.normalize();
                    let is_sprint = speed > 11.0;
                    // Controlled distance in front of running boots
                    let lead_dist = if is_sprint { 1.35 } else { 0.85 };
                    let target_pos = p_tf.translation + move_dir * lead_dist;

                    let to_target = target_pos - ball_tf.translation;
                    // Steer velocity toward target pocket smoothly without snapping/teleporting
                    ball_vel.0 = p_vel.0 + to_target * 7.5;

                    // Physical capsule body collision:
                    if dist < 0.60 && dist > 0.001 {
                        let nx = dx / dist;
                        let nz = dz / dist;
                        let push = 0.60 - dist;
                        ball_tf.translation.x += nx * push;
                        ball_tf.translation.z += nz * push;
                    }
                } else {
                    // Player stopping / stationary: ball cushions to a stop at boots
                    if dist < 1.1 {
                        ball_vel.0 *= 0.80;
                    }
                }
            }
        } else {
            // ── Teammates / Opponents: Physical Collision ────────────
            if !in_kick_cooldown {
                let col_radius = 0.80;
                if dist < col_radius && dist > 0.01 {
                    let nx = dx / dist;
                    let nz = dz / dist;
                    let push = (col_radius - dist) * 2.0;
                    ball_vel.0 += Vec3::new(nx, 0.0, nz) * (push + 2.0);
                }
            }
        }
    }
}

/// Detects goals inside the net and fires a GoalScored reset event after celebration
fn detect_goal(
    time: Res<Time>,
    mut events: EventWriter<SetPieceEvent>,
    mut cooldown: ResMut<GoalCooldown>,
    mut ball_q: Query<(&mut Transform, &mut Velocity), With<Ball>>,
    mut score: ResMut<Score>,
) {
    if cooldown.0 > 0.0 {
        cooldown.0 -= time.delta_seconds();
        if cooldown.0 <= 0.0 {
            // Signal instant formation reset and center spot restart!
            events.send(SetPieceEvent::GoalScored);
        }
        return;
    }

    let Ok((tf, _vel)) = ball_q.get_single_mut() else {
        return;
    };

    let past_line = tf.translation.x.abs() > HALF_LEN + 0.3;
    let between_posts = tf.translation.z.abs() < (GOAL_WIDTH * 0.5) - 0.1;
    let under_bar = tf.translation.y < GOAL_HEIGHT;

    if past_line && between_posts && under_bar {
        if tf.translation.x > 0.0 {
            score.home += 1;
        } else {
            score.away += 1;
        }
        // Crisp 0.35s net ripple before instant formation reset
        cooldown.0 = 0.35;
    }
}

/// Clears goal kicks taken by the away goalkeeper back into midfield
fn handle_away_goal_kick(
    time: Res<Time>,
    mut away_timer: ResMut<AwayGoalKickTimer>,
    mut ball_q: Query<(&mut Velocity, &mut BallKickCooldown, &mut Ball), With<Ball>>,
) {
    let Some(ref mut timer) = away_timer.0 else {
        return;
    };
    *timer -= time.delta_seconds();
    if *timer <= 0.0 {
        away_timer.0 = None;
        if let Ok((mut vel, mut kick_cd, mut ball)) = ball_q.get_single_mut() {
            // Away goalkeeper boots ball downfield into midfield
            vel.0 = Vec3::new(-24.0, 0.0, 0.0);
            kick_cd.0 = 0.5;
            ball.is_out_of_bounds = false;
        }
    }
}
