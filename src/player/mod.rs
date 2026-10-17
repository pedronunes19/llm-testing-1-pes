use bevy::prelude::*;
use crate::ball::{Ball, BallKickCooldown, BALL_RADIUS};
use crate::game::{GameState, SetPieceEvent, Team, Velocity};
use crate::pitch::{HALF_LEN, HALF_WID};

// ── Plugin ──────────────────────────────────────────────────────────

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_players, spawn_marker))
            .add_systems(
                Update,
                (
                    handle_set_pieces,
                    handle_player_switch,
                    reposition_uncontrolled_players,
                    apply_player_velocity,
                    clamp_players_to_pitch,
                    update_controlled_marker,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

// ── Components ──────────────────────────────────────────────────────

#[derive(Component)]
#[allow(dead_code)]
pub struct Player {
    pub team: Team,
    pub role: Role,
    pub base_pos: Vec2,
    pub speed: f32,
}

/// Marker: the player currently controlled by the human.
#[derive(Component)]
pub struct Controlled;

/// Visual overhead indicator (PES arrow) hovering directly above the controlled player.
#[derive(Component)]
pub struct ControlledMarker;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Goalkeeper,
    Defender,
    Midfielder,
    Forward,
}

// ── 4-4-2 formation (x, z) — Home attacks toward +X ────────────────

struct Pos(f32, f32, Role);

const HOME_442: [Pos; 11] = [
    Pos(-48.0,   0.0, Role::Goalkeeper),
    Pos(-34.0, -20.0, Role::Defender),
    Pos(-35.0,  -7.0, Role::Defender),
    Pos(-35.0,   7.0, Role::Defender),
    Pos(-34.0,  20.0, Role::Defender),
    Pos(-18.0, -22.0, Role::Midfielder),
    Pos(-18.0,  -7.0, Role::Midfielder),
    Pos(-18.0,   7.0, Role::Midfielder),
    Pos(-18.0,  22.0, Role::Midfielder),
    Pos( -5.0,  -7.0, Role::Forward), // Striker 1 (Canonical base position)
    Pos( -5.0,   7.0, Role::Forward), // Striker 2 (Canonical base position)
];

// ── Startup ─────────────────────────────────────────────────────────

fn spawn_players(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let capsule = meshes.add(Capsule3d::new(0.4, 1.0));

    let home_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.15, 0.15),
        perceptual_roughness: 0.5,
        ..default()
    });
    let away_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.35, 0.95),
        perceptual_roughness: 0.5,
        ..default()
    });

    // Home team
    for (i, p) in HOME_442.iter().enumerate() {
        let spawn_x = if i == 9 { -1.2 } else { p.0 };
        let spawn_z = if i == 9 { 0.0 } else { p.1 };

        let mut cmd = commands.spawn((
            PbrBundle {
                mesh: capsule.clone(),
                material: home_mat.clone(),
                transform: Transform::from_xyz(spawn_x, 0.9, spawn_z),
                ..default()
            },
            Player {
                team: Team::Home,
                role: p.2,
                base_pos: Vec2::new(p.0, p.1),
                speed: 10.0,
            },
            Velocity::default(),
        ));
        if i == 9 {
            cmd.insert(Controlled);
        }
    }

    // Away team — mirrored
    for (i, p) in HOME_442.iter().enumerate() {
        let x = if i == 9 { 5.0 } else { -p.0 };
        let z = -p.1;

        commands.spawn((
            PbrBundle {
                mesh: capsule.clone(),
                material: away_mat.clone(),
                transform: Transform::from_xyz(x, 0.9, z),
                ..default()
            },
            Player {
                team: Team::Away,
                role: p.2,
                base_pos: Vec2::new(-p.0, -p.1),
                speed: 8.5,
            },
            Velocity::default(),
        ));
    }
}

/// Spawns a small, clean yellow arrow (PES player cursor) pointing down at the player
fn spawn_marker(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cone {
                radius: 0.18,
                height: 0.32,
            }),
            material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.98, 0.82, 0.08),
                unlit: true,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, 2.05, 0.0)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::PI)),
            ..default()
        },
        ControlledMarker,
    ));
}

// ── Systems ─────────────────────────────────────────────────────────

/// Update small yellow arrow to stay right on top of the controlled player's head
fn update_controlled_marker(
    controlled_q: Query<&Transform, (With<Controlled>, Without<ControlledMarker>)>,
    mut marker_q: Query<&mut Transform, With<ControlledMarker>>,
    time: Res<Time>,
) {
    let Ok(player_tf) = controlled_q.get_single() else {
        return;
    };
    let Ok(mut marker_tf) = marker_q.get_single_mut() else {
        return;
    };

    let bob = (time.elapsed_seconds() * 4.0).sin() * 0.03;
    marker_tf.translation = player_tf.translation + Vec3::new(0.0, 2.05 + bob, 0.0);
    marker_tf.rotation = Quat::from_rotation_x(std::f32::consts::PI);
}

/// Handles Throw-ins, Corners, Goal Kicks, and instant formation reset on Goals
fn handle_set_pieces(
    mut events: EventReader<SetPieceEvent>,
    mut commands: Commands,
    mut ball_q: Query<(&mut Transform, &mut Velocity, &mut BallKickCooldown, &mut Ball)>,
    mut players_q: Query<(Entity, &mut Transform, &mut Velocity, &Player, Option<&Controlled>), Without<Ball>>,
) {
    for event in events.read() {
        let Ok((mut ball_tf, mut ball_vel, mut kick_cd, mut ball)) = ball_q.get_single_mut() else {
            continue;
        };

        match *event {
            SetPieceEvent::GoalScored => {
                // 1. Reset ball to center spot for kickoff
                ball_tf.translation = Vec3::new(0.0, BALL_RADIUS, 0.0);
                ball_vel.0 = Vec3::ZERO;
                kick_cd.0 = 0.0;
                ball.is_out_of_bounds = false;

                // 2. Reset ALL 22 players INSTANTLY to their original starting formation positions!
                for (entity, mut p_tf, mut p_vel, player, controlled) in &mut players_q {
                    p_vel.0 = Vec3::ZERO;
                    if player.team == Team::Home {
                        // Striker 1 (starts at kickoff touching the ball)
                        if (player.base_pos - Vec2::new(-5.0, -7.0)).length() < 0.1 {
                            p_tf.translation = Vec3::new(-1.2, 0.9, 0.0);
                            commands.entity(entity).insert(Controlled);
                        } else {
                            p_tf.translation = Vec3::new(player.base_pos.x, 0.9, player.base_pos.y);
                            if controlled.is_some() {
                                commands.entity(entity).remove::<Controlled>();
                            }
                        }
                    } else {
                        // Away team
                        if (player.base_pos - Vec2::new(5.0, 7.0)).length() < 0.1 {
                            p_tf.translation = Vec3::new(5.0, 0.9, 0.0);
                        } else {
                            p_tf.translation = Vec3::new(player.base_pos.x, 0.9, player.base_pos.y);
                        }
                        if controlled.is_some() {
                            commands.entity(entity).remove::<Controlled>();
                        }
                    }
                }
            }
            SetPieceEvent::ThrowIn { x, z_sign } => {
                // Place ball directly on the touchline
                ball_tf.translation = Vec3::new(x, BALL_RADIUS, z_sign * (HALF_WID - 0.25));
                ball_vel.0 = Vec3::ZERO;
                kick_cd.0 = 0.4;

                let target_spot = Vec3::new(x, 0.9, z_sign * (HALF_WID + 0.5));
                let mut best_entity = None;
                let mut min_d = f32::MAX;

                for (entity, p_tf, _, player, _) in &players_q {
                    if player.team == Team::Home && player.role != Role::Goalkeeper {
                        let d = p_tf.translation.distance(target_spot);
                        if d < min_d {
                            min_d = d;
                            best_entity = Some(entity);
                        }
                    }
                }

                // Place thrower on sideline behind ball and give them control
                for (entity, mut p_tf, mut p_vel, _, controlled) in &mut players_q {
                    if Some(entity) == best_entity {
                        p_tf.translation = target_spot;
                        p_vel.0 = Vec3::ZERO;
                        commands.entity(entity).insert(Controlled);
                    } else if controlled.is_some() {
                        commands.entity(entity).remove::<Controlled>();
                    }
                }
            }
            SetPieceEvent::CornerKick { x_sign, z_sign } => {
                // Place ball directly inside the corner arc
                ball_tf.translation = Vec3::new(
                    x_sign * (HALF_LEN - 0.7),
                    BALL_RADIUS,
                    z_sign * (HALF_WID - 0.7),
                );
                ball_vel.0 = Vec3::ZERO;
                kick_cd.0 = 0.4;

                let corner_taker_pos = Vec3::new(
                    x_sign * (HALF_LEN + 0.3),
                    0.9,
                    z_sign * (HALF_WID + 0.3),
                );
                let mut best_entity = None;
                let mut min_d = f32::MAX;

                for (entity, p_tf, _, player, _) in &players_q {
                    if player.team == Team::Home && player.role != Role::Goalkeeper {
                        let d = p_tf.translation.distance(corner_taker_pos);
                        if d < min_d {
                            min_d = d;
                            best_entity = Some(entity);
                        }
                    }
                }

                for (entity, mut p_tf, mut p_vel, _, controlled) in &mut players_q {
                    if Some(entity) == best_entity {
                        p_tf.translation = corner_taker_pos;
                        p_vel.0 = Vec3::ZERO;
                        commands.entity(entity).insert(Controlled);
                    } else if controlled.is_some() {
                        commands.entity(entity).remove::<Controlled>();
                    }
                }
            }
            SetPieceEvent::GoalKick { x_sign } => {
                // Goal kick placed at the edge of the 6-yard box
                let gk_ball_pos = Vec3::new(x_sign * (HALF_LEN - 5.5), BALL_RADIUS, 0.0);
                ball_tf.translation = gk_ball_pos;
                ball_vel.0 = Vec3::ZERO;
                kick_cd.0 = 0.4;

                let is_home = x_sign < 0.0;
                let target_team = if is_home { Team::Home } else { Team::Away };

                for (entity, mut p_tf, mut p_vel, player, controlled) in &mut players_q {
                    if player.team == target_team && player.role == Role::Goalkeeper {
                        p_tf.translation = Vec3::new(x_sign * (HALF_LEN - 4.2), 0.9, 0.0);
                        p_vel.0 = Vec3::ZERO;
                        if is_home {
                            commands.entity(entity).insert(Controlled);
                        }
                    } else if is_home && controlled.is_some() {
                        commands.entity(entity).remove::<Controlled>();
                    }
                }
            }
        }
    }
}

/// Non-controlled players dynamically move to their base position adjusted by the ball location
fn reposition_uncontrolled_players(
    ball_q: Query<(&Transform, &Ball)>,
    mut query: Query<(&Player, &Transform, &mut Velocity), Without<Controlled>>,
) {
    let Ok((ball_tf, ball)) = ball_q.get_single() else {
        return;
    };

    // If ball is out of bounds awaiting a set piece, hold formation
    if ball.is_out_of_bounds {
        return;
    }

    let bx = ball_tf.translation.x;
    let bz = ball_tf.translation.z;

    for (player, p_tf, mut vel) in &mut query {
        if player.team != Team::Home {
            vel.0 = Vec3::ZERO;
            continue;
        }

        let (target_x, target_z) = match player.role {
            Role::Goalkeeper => {
                let tx = (-48.0 + (bx + 40.0).max(0.0) * 0.06).clamp(-49.0, -43.0);
                let tz = (bz * 0.25).clamp(-3.2, 3.2);
                (tx, tz)
            }
            Role::Defender => {
                let tx = (player.base_pos.x + bx * 0.40).clamp(-44.0, -2.0);
                let tz = (player.base_pos.y + bz * 0.18).clamp(-HALF_WID + 2.0, HALF_WID - 2.0);
                (tx, tz)
            }
            Role::Midfielder => {
                let tx = (player.base_pos.x + bx * 0.50).clamp(-34.0, 28.0);
                let tz = (player.base_pos.y + bz * 0.22).clamp(-HALF_WID + 2.0, HALF_WID - 2.0);
                (tx, tz)
            }
            Role::Forward => {
                let tx = (player.base_pos.x + bx * 0.60).clamp(-22.0, 46.0);
                let tz = (player.base_pos.y + bz * 0.20).clamp(-HALF_WID + 3.0, HALF_WID - 3.0);
                (tx, tz)
            }
        };

        let target = Vec3::new(target_x, p_tf.translation.y, target_z);
        let delta = target - p_tf.translation;
        let dist = delta.length();

        if dist > 0.8 {
            let speed = if dist > 10.0 {
                player.speed * 0.70
            } else if dist > 4.0 {
                player.speed * 0.50
            } else {
                player.speed * 0.30
            };
            vel.0 = delta.normalize() * speed;
        } else {
            vel.0 = Vec3::ZERO;
        }
    }
}

/// Switch controlled player to the teammate closest to the ball with Q or Tab
fn handle_player_switch(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    ball_q: Query<&Transform, With<Ball>>,
    home_players_q: Query<(Entity, &Transform, &Player), Without<Ball>>,
    controlled_q: Query<Entity, With<Controlled>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyQ) && !keyboard.just_pressed(KeyCode::Tab) {
        return;
    }

    let Ok(ball_tf) = ball_q.get_single() else {
        return;
    };
    let current_controlled = controlled_q.get_single().ok();

    let mut closest_entity = None;
    let mut min_dist = f32::MAX;

    for (entity, tf, player) in &home_players_q {
        if player.team != Team::Home || Some(entity) == current_controlled {
            continue;
        }
        let d = tf.translation.distance(ball_tf.translation);
        if d < min_dist {
            min_dist = d;
            closest_entity = Some(entity);
        }
    }

    if let Some(new_target) = closest_entity {
        if let Some(prev) = current_controlled {
            commands.entity(prev).remove::<Controlled>();
        }
        commands.entity(new_target).insert(Controlled);
    }
}

fn apply_player_velocity(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Velocity), With<Player>>,
) {
    let dt = time.delta_seconds();
    for (mut tf, vel) in &mut query {
        tf.translation += vel.0 * dt;
        if vel.0.length_squared() > 0.2 {
            let target_angle = (-vel.0.z).atan2(vel.0.x);
            let target_rot = Quat::from_rotation_y(target_angle);
            tf.rotation = tf.rotation.slerp(target_rot, (14.0 * dt).min(1.0));
        }
    }
}

fn clamp_players_to_pitch(mut query: Query<&mut Transform, With<Player>>) {
    for mut tf in &mut query {
        tf.translation.x = tf.translation.x.clamp(-HALF_LEN - 3.0, HALF_LEN + 3.0);
        tf.translation.z = tf.translation.z.clamp(-HALF_WID - 3.0, HALF_WID + 3.0);
    }
}
