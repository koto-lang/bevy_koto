use bevy::{
    core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping},
    prelude::*,
    render::camera::ScalingMode,
    window::WindowResized,
};
use cloned::cloned;
use koto::prelude::*;

use crate::runtime::{
    make_channel, KotoReceiver, KotoRuntime, KotoRuntimePlugin, KotoSchedule, KotoSender,
    KotoUpdate, ScriptLoaded,
};

pub struct KotoCameraPlugin;

impl Plugin for KotoCameraPlugin {
    fn build(&self, app: &mut App) {
        debug_assert!(app.is_plugin_added::<KotoRuntimePlugin>());

        let (update_ortho_projection_sender, update_ortho_projection_receiver) =
            make_channel::<UpdateOrthoProjection>();

        app.insert_resource(update_ortho_projection_sender)
            .insert_resource(update_ortho_projection_receiver)
            .add_systems(Startup, setup_camera)
            .add_systems(Startup, setup_koto)
            .add_systems(KotoSchedule, on_script_loaded.in_set(KotoUpdate::PreUpdate))
            .add_systems(Update, (on_window_resized, update_orthographic_projection));
    }
}

#[derive(Clone, Event)]
pub enum UpdateOrthoProjection {
    Scale(f32),
}

pub type UpdateOrthoProjectionSender = KotoSender<UpdateOrthoProjection>;
type UpdateOrthoProjectionReceiver = KotoReceiver<UpdateOrthoProjection>;

/// Used to help identify our main camera
#[derive(Component)]
pub struct MainCamera;

fn setup_camera(mut commands: Commands) {
    commands
        .spawn((Camera2dBundle { ..default() }, BloomSettings::default()))
        .insert(MainCamera);
}

fn setup_koto(koto: Res<KotoRuntime>, update_projection: Res<UpdateOrthoProjectionSender>) {
    koto.prelude().add_fn("set_zoom", {
        cloned!(update_projection);
        move |ctx| match ctx.args() {
            [KValue::Number(zoom)] => {
                update_projection.send(UpdateOrthoProjection::Scale(zoom.into()));
                Ok(KValue::Null)
            }
            unexpected => type_error_with_slice("a Number", unexpected),
        }
    });
}

// Reset the camera projection when a new script is loaded
fn on_script_loaded(
    mut script_loaded_events: EventReader<ScriptLoaded>,
    mut camera_query: Query<&mut OrthographicProjection, With<MainCamera>>,
) {
    for _ in script_loaded_events.read() {
        let mut camera = camera_query.single_mut();
        camera.scale = 1.0;
    }
}

fn update_orthographic_projection(
    channel: Res<UpdateOrthoProjectionReceiver>,
    mut camera_query: Query<&mut OrthographicProjection, With<MainCamera>>,
) {
    let mut camera = camera_query.single_mut();
    while let Some(event) = channel.receive() {
        match event {
            UpdateOrthoProjection::Scale(scale) => camera.scale = scale,
        }
    }
}

fn on_window_resized(
    mut window_resized_events: EventReader<WindowResized>,
    mut camera_query: Query<&mut OrthographicProjection, With<MainCamera>>,
) {
    let mut camera = camera_query.single_mut();
    for event in window_resized_events.read() {
        camera.scaling_mode = get_scaling_mode(event.width, event.height);
    }
}

fn get_scaling_mode(width: f32, height: f32) -> ScalingMode {
    if width > height {
        ScalingMode::FixedVertical(2.0)
    } else {
        ScalingMode::FixedHorizontal(2.0)
    }
}
