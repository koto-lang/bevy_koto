//! # bevy_koto
//!
//! Plugins for Bevy that add support for scripting with Koto.

#![warn(missing_docs)]

mod camera;
mod color;
mod entity;
mod geometry;
mod random;
mod runtime;
mod shape;
mod text;
mod window;

pub use koto;
pub use koto_color;
pub use koto_geometry;
pub use koto_random;

pub use {
    camera::{KotoCamera, KotoCameraPlugin, UpdateOrthographicProjection},
    color::{koto_to_bevy_color, KotoColor, KotoColorPlugin, SetClearColor, UpdateColorMaterial},
    entity::{
        koto_entity_channel, KotoEntity, KotoEntityEvent, KotoEntityMapping, KotoEntityPlugin,
        KotoEntityReceiver, KotoEntitySender, UpdateEntity,
    },
    geometry::{KotoGeometryPlugin, KotoVec2, UpdateTransform},
    random::KotoRandomPlugin,
    runtime::{
        koto_channel, KotoReceiver, KotoRuntime, KotoRuntimePlugin, KotoSchedule, KotoScript,
        KotoSender, KotoUpdate, LoadScript, ScriptLoaded,
    },
    shape::KotoShapePlugin,
    text::KotoTextPlugin,
    window::KotoWindowPlugin,
};
