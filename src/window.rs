use crate::{KotoRuntime, KotoRuntimePlugin, KotoSchedule, KotoUpdate, ScriptLoaded};
use bevy::{
    prelude::*,
    window::{PrimaryWindow, WindowResized},
};

/// Window events for bevy_koto
///
/// The plugin currently only detects window resize events, and then calls the script's
/// exported `on_window_size` function (if it exists).
pub struct KotoWindowPlugin;

impl Plugin for KotoWindowPlugin {
    fn build(&self, app: &mut App) {
        debug_assert!(app.is_plugin_added::<KotoRuntimePlugin>());

        app.add_systems(
            KotoSchedule,
            (on_script_compiled, on_window_resized).in_set(KotoUpdate::PreUpdate),
        );
    }
}

fn on_script_compiled(
    mut koto: ResMut<KotoRuntime>,
    mut script_loaded_events: EventReader<ScriptLoaded>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
) {
    for _ in script_loaded_events.read() {
        if let Ok(window) = primary_window.get_single() {
            run_on_window_size(&mut koto, window.width(), window.height());
        } else {
            error!("Missing primary window");
        }
    }
}

fn on_window_resized(
    mut koto: ResMut<KotoRuntime>,
    mut window_resized_events: EventReader<WindowResized>,
) {
    for event in window_resized_events.read() {
        run_on_window_size(&mut koto, event.width, event.height);
    }
}

fn run_on_window_size(koto: &mut KotoRuntime, width: f32, height: f32) {
    if koto.is_ready() {
        if let Err(error) = koto.run_exported_function(
            "on_window_size",
            &[koto.user_data().clone(), width.into(), height.into()],
        ) {
            error!("Error in 'on_window_size':\n{error}");
        }
    }
}
