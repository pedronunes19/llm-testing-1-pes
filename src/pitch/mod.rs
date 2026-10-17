use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, TAU};

// ── Plugin ──────────────────────────────────────────────────────────

pub struct PitchPlugin;

impl Plugin for PitchPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_pitch);
    }
}

// ── Dimensions (metres, 1 unit = 1 m) ──────────────────────────────

pub const PITCH_LENGTH: f32 = 105.0;
pub const PITCH_WIDTH: f32 = 68.0;
pub const HALF_LEN: f32 = PITCH_LENGTH / 2.0;
pub const HALF_WID: f32 = PITCH_WIDTH / 2.0;
pub const GOAL_WIDTH: f32 = 7.32;
pub const GOAL_HEIGHT: f32 = 2.44;
pub const GOAL_DEPTH: f32 = 2.20;

// ── Setup ───────────────────────────────────────────────────────────

fn setup_pitch(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // ── Grass surface ───────────────────────────────────────────────
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(PITCH_LENGTH + 16.0, 0.1, PITCH_WIDTH + 14.0)),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.14, 0.50, 0.15),
            perceptual_roughness: 0.95,
            ..default()
        }),
        transform: Transform::from_xyz(0.0, -0.05, 0.0),
        ..default()
    });

    let line_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        ..default()
    });
    let lw: f32 = 0.12;
    let lh: f32 = 0.005;

    // ── Boundary lines ──────────────────────────────────────────────
    // Sidelines (run along X)
    for &z in &[-HALF_WID, HALF_WID] {
        spawn_box(&mut commands, &mut meshes, &line_mat,
                   PITCH_LENGTH, lh, lw, 0.0, 0.005, z);
    }
    // Goal lines (run along Z)
    for &x in &[-HALF_LEN, HALF_LEN] {
        spawn_box(&mut commands, &mut meshes, &line_mat,
                   lw, lh, PITCH_WIDTH, x, 0.005, 0.0);
    }

    // ── Centre line ─────────────────────────────────────────────────
    spawn_box(&mut commands, &mut meshes, &line_mat,
               lw, lh, PITCH_WIDTH, 0.0, 0.005, 0.0);

    // ── Centre circle ───────────────────────────────────────────────
    let radius: f32 = 9.15;
    let segs: usize = 48;
    for i in 0..segs {
        let a1 = (i as f32 / segs as f32) * TAU;
        let a2 = ((i + 1) as f32 / segs as f32) * TAU;
        let (x1, z1) = (radius * a1.cos(), radius * a1.sin());
        let (x2, z2) = (radius * a2.cos(), radius * a2.sin());
        let mx = (x1 + x2) * 0.5;
        let mz = (z1 + z2) * 0.5;
        let seg_len = ((x2 - x1).powi(2) + (z2 - z1).powi(2)).sqrt();
        let angle = (z2 - z1).atan2(x2 - x1);
        commands.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(seg_len, lh, lw)),
            material: line_mat.clone(),
            transform: Transform::from_xyz(mx, 0.006, mz)
                .with_rotation(Quat::from_rotation_y(-angle)),
            ..default()
        });
    }

    // Centre spot
    commands.spawn(PbrBundle {
        mesh: meshes.add(Sphere::new(0.18)),
        material: line_mat.clone(),
        transform: Transform::from_xyz(0.0, 0.01, 0.0),
        ..default()
    });

    // ── Penalty areas ───────────────────────────────────────────────
    let pa_depth: f32 = 16.5;
    let pa_width: f32 = 40.32;
    for &x_sign in &[-1.0_f32, 1.0] {
        let front_x = x_sign * (HALF_LEN - pa_depth);
        spawn_box(&mut commands, &mut meshes, &line_mat,
                   lw, lh, pa_width, front_x, 0.005, 0.0);
        let cx = x_sign * (HALF_LEN - pa_depth * 0.5);
        for &z_sign in &[-1.0_f32, 1.0] {
            spawn_box(&mut commands, &mut meshes, &line_mat,
                       pa_depth, lh, lw, cx, 0.005, z_sign * pa_width * 0.5);
        }
    }

    // ── Authentic Stadium Box Goals & Netting ───────────────────────
    let post_r: f32 = 0.06;
    let goal_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.96, 0.96, 0.96),
        metallic: 0.85,
        perceptual_roughness: 0.25,
        ..default()
    });
    let net_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.92, 0.94, 0.96, 0.42),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        double_sided: true,
        unlit: true,
        ..default()
    });

    let upright_mesh = meshes.add(Cylinder::new(post_r, GOAL_HEIGHT * 0.5));
    let crossbar_mesh = meshes.add(Cylinder::new(post_r, GOAL_WIDTH * 0.5));
    let side_bar_mesh = meshes.add(Cylinder::new(post_r, GOAL_DEPTH * 0.5));

    for &x_sign in &[-1.0_f32, 1.0] {
        let gx = x_sign * HALF_LEN;
        let back_x = gx + x_sign * GOAL_DEPTH;
        let mid_x = (gx + back_x) * 0.5;

        // 1. Front Upright Posts (full height from pitch to crossbar)
        for &z_sign in &[-1.0_f32, 1.0] {
            commands.spawn(PbrBundle {
                mesh: upright_mesh.clone(),
                material: goal_mat.clone(),
                transform: Transform::from_xyz(gx, GOAL_HEIGHT * 0.5, z_sign * GOAL_WIDTH * 0.5),
                ..default()
            });
        }

        // 2. Front Crossbar
        commands.spawn(PbrBundle {
            mesh: crossbar_mesh.clone(),
            material: goal_mat.clone(),
            transform: Transform::from_xyz(gx, GOAL_HEIGHT, 0.0)
                .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
            ..default()
        });

        // 3. Rear Upright Posts
        for &z_sign in &[-1.0_f32, 1.0] {
            commands.spawn(PbrBundle {
                mesh: upright_mesh.clone(),
                material: goal_mat.clone(),
                transform: Transform::from_xyz(back_x, GOAL_HEIGHT * 0.5, z_sign * GOAL_WIDTH * 0.5),
                ..default()
            });
        }

        // 4. Rear Top Crossbar
        commands.spawn(PbrBundle {
            mesh: crossbar_mesh.clone(),
            material: goal_mat.clone(),
            transform: Transform::from_xyz(back_x, GOAL_HEIGHT, 0.0)
                .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
            ..default()
        });

        // 5. Rear Bottom Ground Bar
        commands.spawn(PbrBundle {
            mesh: crossbar_mesh.clone(),
            material: goal_mat.clone(),
            transform: Transform::from_xyz(back_x, 0.05, 0.0)
                .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
            ..default()
        });

        // 6. Top & Ground Horizontal Side Bars
        for &z_sign in &[-1.0_f32, 1.0] {
            let gz = z_sign * GOAL_WIDTH * 0.5;
            // Top side bar
            commands.spawn(PbrBundle {
                mesh: side_bar_mesh.clone(),
                material: goal_mat.clone(),
                transform: Transform::from_xyz(mid_x, GOAL_HEIGHT, gz)
                    .with_rotation(Quat::from_rotation_z(FRAC_PI_2)),
                ..default()
            });
            // Bottom side bar on ground
            commands.spawn(PbrBundle {
                mesh: side_bar_mesh.clone(),
                material: goal_mat.clone(),
                transform: Transform::from_xyz(mid_x, 0.05, gz)
                    .with_rotation(Quat::from_rotation_z(FRAC_PI_2)),
                ..default()
            });
        }

        // 7. Box Netting Panels (Top, Back, Left Side, Right Side)
        // Back net
        commands.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(0.02, GOAL_HEIGHT, GOAL_WIDTH)),
            material: net_mat.clone(),
            transform: Transform::from_xyz(back_x, GOAL_HEIGHT * 0.5, 0.0),
            ..default()
        });
        // Top net (roof)
        commands.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(GOAL_DEPTH, 0.02, GOAL_WIDTH)),
            material: net_mat.clone(),
            transform: Transform::from_xyz(mid_x, GOAL_HEIGHT, 0.0),
            ..default()
        });
        // Left & Right side nets
        for &z_sign in &[-1.0_f32, 1.0] {
            commands.spawn(PbrBundle {
                mesh: meshes.add(Cuboid::new(GOAL_DEPTH, GOAL_HEIGHT, 0.02)),
                material: net_mat.clone(),
                transform: Transform::from_xyz(mid_x, GOAL_HEIGHT * 0.5, z_sign * GOAL_WIDTH * 0.5),
                ..default()
            });
        }
    }

    // ── Lighting ────────────────────────────────────────────────────
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 14_000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -FRAC_PI_4,
            FRAC_PI_4,
            0.0,
        )),
        ..default()
    });

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 450.0,
    });
}

// ── Helpers ─────────────────────────────────────────────────────────

fn spawn_box(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mat: &Handle<StandardMaterial>,
    sx: f32, sy: f32, sz: f32,
    x: f32, y: f32, z: f32,
) {
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(sx, sy, sz)),
        material: mat.clone(),
        transform: Transform::from_xyz(x, y, z),
        ..default()
    });
}
