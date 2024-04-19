use crate::{
    entity::{KotoEntityEvent, KotoEntityPlugin},
    runtime::{make_channel, KotoReceiver, KotoRuntime, KotoRuntimePlugin, KotoSender},
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
            make_channel::<UpdateTransformEvent>();

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

fn update_transform(channel: Res<UpdateTransformReceiver>, mut q: Query<&mut Transform>) {
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

#[derive(Clone, Event)]
pub enum UpdateTransform {
    Position(Vec3),
    Rotation(f32),
    Scale(Vec3),
}

pub type UpdateTransformEvent = KotoEntityEvent<UpdateTransform>;
pub type UpdateTransformSender = KotoSender<UpdateTransformEvent>;
type UpdateTransformReceiver = KotoReceiver<UpdateTransformEvent>;
