use bevy::prelude::*;
use crate::ball::Ball;

// ── Plugin ──────────────────────────────────────────────────────────

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CameraFocus::default())
            .add_systems(Startup, setup_camera)
            .add_systems(Update, follow_ball);
    }
}

// ── Components & Resources ──────────────────────────────────────────

#[derive(Component)]
struct MainCamera;

/// Filtered focus point that smoothly tracks the ball to eliminate sudden camera snaps
#[derive(Resource)]
pub struct CameraFocus(pub Vec3);

impl Default for CameraFocus {
    fn default() -> Self {
        Self(Vec3::ZERO)
    }
}

// ── Systems ─────────────────────────────────────────────────────────

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 31.0, 43.0)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        MainCamera,
    ));
}

/// Broadcast-style camera that smoothly tracks the ball from the sideline with filtered pivot
fn follow_ball(
    time: Res<Time>,
    ball_q: Query<&Transform, (With<Ball>, Without<MainCamera>)>,
    mut cam_q: Query<&mut Transform, With<MainCamera>>,
    mut focus: ResMut<CameraFocus>,
) {
    let Ok(ball_tf) = ball_q.get_single() else {
        return;
    };
    let Ok(mut cam_tf) = cam_q.get_single_mut() else {
        return;
    };

    let dt = time.delta_seconds();

    // 1. Smoothly filter the focus look-at target so 180° turns and sudden kicks don't snap the view
    let focus_t = (3.5 * dt).min(1.0);
    focus.0 = focus.0.lerp(ball_tf.translation, focus_t);

    // 2. Smoothly follow from the sideline gantry
    let offset = Vec3::new(0.0, 31.0, 43.0);
    let target_cam_pos = focus.0 + offset;
    let pos_t = (2.6 * dt).min(1.0);
    cam_tf.translation = cam_tf.translation.lerp(target_cam_pos, pos_t);

    // 3. Aim at the smoothly interpolated focus point
    cam_tf.look_at(focus.0, Vec3::Y);
}
