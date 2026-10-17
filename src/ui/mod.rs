use bevy::prelude::*;
use crate::game::{GameState, MatchClock, Score};

// ── Plugin ──────────────────────────────────────────────────────────

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui)
            .add_systems(
                Update,
                update_scoreboard.run_if(in_state(GameState::Playing)),
            );
    }
}

// ── Components ──────────────────────────────────────────────────────

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct ClockText;

// ── Startup ─────────────────────────────────────────────────────────

fn setup_ui(mut commands: Commands) {
    // Root bar at the top
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(8.0)),
                column_gap: Val::Px(24.0),
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                ..default()
            },
            ..default()
        })
        .with_children(|row| {
            // HOME score
            row.spawn((
                TextBundle::from_section(
                    "HOME  0",
                    TextStyle {
                        font_size: 28.0,
                        color: Color::srgb(1.0, 0.3, 0.3),
                        ..default()
                    },
                ),
                ScoreText,
            ));

            // Clock
            row.spawn((
                TextBundle::from_section(
                    "00:00",
                    TextStyle {
                        font_size: 28.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ),
                ClockText,
            ));

            // AWAY score (we identify it via ScoreText too — update fn writes both)
        });

    // Controls hint at the bottom
    commands.spawn(
        TextBundle::from_section(
            "WASD: Move  |  Shift: Sprint  |  X / K: Pass  |  Space: Shoot  |  Q: Switch",
            TextStyle {
                font_size: 16.0,
                color: Color::srgba(1.0, 1.0, 1.0, 0.5),
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(0.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_text_justify(JustifyText::Center),
    );
}

// ── Update ──────────────────────────────────────────────────────────

fn update_scoreboard(
    score: Res<Score>,
    clock: Res<MatchClock>,
    mut score_q: Query<&mut Text, (With<ScoreText>, Without<ClockText>)>,
    mut clock_q: Query<&mut Text, With<ClockText>>,
) {
    for mut text in &mut score_q {
        text.sections[0].value = format!("HOME {}  —  {} AWAY", score.home, score.away);
    }
    for mut text in &mut clock_q {
        text.sections[0].value = clock.display();
    }
}
