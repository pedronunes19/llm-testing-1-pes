use bevy::prelude::*;

// ── Plugin ──────────────────────────────────────────────────────────

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .add_event::<SetPieceEvent>()
            .insert_resource(MatchClock::default())
            .insert_resource(Score::default())
            .add_systems(
                Update,
                tick_match_clock.run_if(in_state(GameState::Playing)),
            );
    }
}

// ── States & Events ─────────────────────────────────────────────────

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Playing,
}

/// Set-piece restart events for throw-ins, corners, goal kicks, and goal resets
#[derive(Event, Debug, Clone, Copy)]
pub enum SetPieceEvent {
    ThrowIn { x: f32, z_sign: f32 },
    CornerKick { x_sign: f32, z_sign: f32 },
    GoalKick { x_sign: f32 },
    GoalScored,
}

// ── Shared components ───────────────────────────────────────────────

/// Velocity vector for any moving entity (players, ball).
#[derive(Component, Default, Clone, Debug)]
pub struct Velocity(pub Vec3);

/// Team allegiance.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Team {
    Home,
    Away,
}

// ── Resources ───────────────────────────────────────────────────────

/// Match clock — elapsed time in "match seconds".
#[derive(Resource)]
pub struct MatchClock {
    pub elapsed_secs: f32,
    pub half: u8,
}

impl Default for MatchClock {
    fn default() -> Self {
        Self {
            elapsed_secs: 0.0,
            half: 1,
        }
    }
}

impl MatchClock {
    /// Current match minute (0‥90+).
    pub fn minute(&self) -> u32 {
        let base: u32 = if self.half == 1 { 0 } else { 45 };
        base + (self.elapsed_secs / 60.0) as u32
    }

    /// Formatted `MM:SS` for the scoreboard.
    pub fn display(&self) -> String {
        let min = self.minute();
        let sec = (self.elapsed_secs % 60.0) as u32;
        format!("{min:02}:{sec:02}")
    }
}

/// Match score.
#[derive(Resource, Default)]
pub struct Score {
    pub home: u32,
    pub away: u32,
}

// ── Systems ─────────────────────────────────────────────────────────

/// Match time advances at 2× real time → a 45-min half takes ~22 min.
fn tick_match_clock(time: Res<Time>, mut clock: ResMut<MatchClock>) {
    clock.elapsed_secs += time.delta_seconds() * 2.0;
}
