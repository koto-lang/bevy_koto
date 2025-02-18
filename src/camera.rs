//! Support for modifying properties of a Bevy camera

use crate::prelude::*;
use bevy::{prelude::*, render::camera::ScalingMode, window::WindowResized};
use cloned::cloned;
use koto::prelude::*;

/// Exposes a `set_zoom` function to Koto that modifies the zoom of a 2D camera
///
/// The camera needs to have the [KotoCamera] component attached to it for the
#[derive(Default)]
pub struct KotoCameraPlugin;

impl Plugin for KotoCameraPlugin {
    fn build(&self, app: &mut App) {
        debug_assert!(app.is_plugin_added::<KotoRuntimePlugin>());

        let (update_ortho_projection_sender, update_ortho_projection_receiver) =
            koto_channel::<UpdateOrthographicProjection>();

        app.insert_resource(update_ortho_projection_sender)
            .insert_resource(update_ortho_projection_receiver)
            .add_systems(Startup, on_startup)
            .add_systems(KotoSchedule, on_script_loaded.in_set(KotoUpdate::PreUpdate))
            .add_systems(Update, (on_window_resized, update_orthographic_projection));
    }
}

/// Event for updating the camera's orthographic projection
#[derive(Clone, Event)]
pub enum UpdateOrthographicProjection {
    /// Sets the projection's scale
    Scale(f32),
}

/// Used to help identify our main camera
#[derive(Component)]
pub struct KotoCamera;

fn on_startup(
    koto: Res<KotoRuntime>,
    update_projection: Res<KotoSender<UpdateOrthographicProjection>>,
) {
    koto.prelude().add_fn("set_zoom", {
        cloned!(update_projection);
        move |ctx| match ctx.args() {
            [KValue::Number(zoom)] => {
                update_projection.send(UpdateOrthographicProjection::Scale(zoom.into()));
                Ok(KValue::Null)
            }
            unexpected => unexpected_args("a Number", unexpected),
        }
    });
}

// Reset the camera's projection when a script is loaded
fn on_script_loaded(
    mut script_loaded_events: EventReader<ScriptLoaded>,
    mut camera_query: Query<&mut OrthographicProjection, With<KotoCamera>>,
) {
    for _ in script_loaded_events.read() {
        let mut camera = camera_query.single_mut();
        camera.scale = 1.0;
    }
}

fn update_orthographic_projection(
    channel: Res<KotoReceiver<UpdateOrthographicProjection>>,
    mut camera_query: Query<&mut OrthographicProjection, With<KotoCamera>>,
) {
    let mut camera = camera_query.single_mut();
    while let Some(event) = channel.receive() {
        match event {
            UpdateOrthographicProjection::Scale(scale) => camera.scale = scale,
        }
    }
}

fn on_window_resized(
    mut window_resized_events: EventReader<WindowResized>,
    mut camera_query: Query<&mut OrthographicProjection, With<KotoCamera>>,
) {
    let mut camera = camera_query.single_mut();
    for event in window_resized_events.read() {
        camera.scaling_mode = get_scaling_mode(event.width, event.height);
    }
}

fn get_scaling_mode(width: f32, height: f32) -> ScalingMode {
    if width > height {
        ScalingMode::FixedVertical {
            viewport_height: 2.0,
        }
    } else {
        ScalingMode::FixedHorizontal {
            viewport_width: 2.0,
        }
    }
}
