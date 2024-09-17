use crate::{
    koto_entity_channel, KotoEntityPlugin, KotoEntityReceiver, KotoRuntime, KotoRuntimePlugin,
};
use bevy::prelude::*;
pub use koto_geometry::Vec2 as KotoVec2;

/// 2d Geometry utilities for Koto
///
/// The plugin adds the `geometry` module from `koto_geometry` to Koto's prelude.
pub struct KotoGeometryPlugin;

impl Plugin for KotoGeometryPlugin {
    fn build(&self, app: &mut App) {
        debug_assert!(app.is_plugin_added::<KotoRuntimePlugin>());
        debug_assert!(app.is_plugin_added::<KotoEntityPlugin>());

        let (update_transform_sender, update_transform_receiver) =
            koto_entity_channel::<UpdateTransform>();

        app.insert_resource(update_transform_sender)
            .insert_resource(update_transform_receiver)
            .add_systems(Startup, on_startup)
            .add_systems(Update, update_transform);
    }
}

fn on_startup(koto: Res<KotoRuntime>) {
    koto.prelude()
        .insert("geometry", koto_geometry::make_module());
}

fn update_transform(
    channel: Res<KotoEntityReceiver<UpdateTransform>>,
    mut q: Query<&mut Transform>,
) {
    while let Some(event) = channel.receive() {
        let mut transform = q.get_mut(event.entity.get()).unwrap();
        match event.event {
            UpdateTransform::Position(position) => transform.translation = position,
            UpdateTransform::Rotation(rotation) => {
                transform.rotation = Quat::from_rotation_z(rotation)
            }
            UpdateTransform::Scale(scale) => transform.scale = scale,
        }
    }
}

/// Event for updating the properties of an entity's transform
#[derive(Clone, Event)]
pub enum UpdateTransform {
    /// Sets the transform's position
    Position(Vec3),
    /// Sets the transform's rotation
    Rotation(f32),
    /// Sets the transform's scale
    Scale(Vec3),
}
