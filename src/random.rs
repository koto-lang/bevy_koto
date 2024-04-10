use crate::runtime::{KotoRuntime, KotoRuntimePlugin};
use bevy::prelude::*;

pub struct KotoRandomPlugin;

impl Plugin for KotoRandomPlugin {
    fn build(&self, app: &mut App) {
        assert!(app.is_plugin_added::<KotoRuntimePlugin>());

        app.add_systems(Startup, on_startup);
    }
}

fn on_startup(koto: Res<KotoRuntime>) {
    koto.prelude().insert("random", koto_random::make_module());
}
