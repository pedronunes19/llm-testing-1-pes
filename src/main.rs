use bevy::prelude::*;

mod ball;
mod camera;
mod game;
mod input;
mod pitch;
mod player;
mod ui;

fn main() {
    // Better panic messages in the browser console.
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Vibe PES 2019".into(),
                    resolution: (1280.0_f32, 720.0_f32).into(),
                    canvas: Some("#game-canvas".into()),
                    fit_canvas_to_parent: true,
                    prevent_default_event_handling: true,
                    ..default()
                }),
                ..default()
            }),
        )
        .add_plugins((
            game::GamePlugin,
            pitch::PitchPlugin,
            player::PlayerPlugin,
            ball::BallPlugin,
            camera::CameraPlugin,
            input::InputPlugin,
            ui::UiPlugin,
        ))
        .run();
}
