//! # bevy_koto
//!
//! Plugins for Bevy that add support for scripting with Koto.

#![warn(missing_docs)]

mod entity;
mod runtime;

#[cfg(feature = "camera")]
mod camera;
#[cfg(feature = "color")]
mod color;
#[cfg(feature = "geometry")]
mod geometry;
#[cfg(feature = "random")]
mod random;
#[cfg(feature = "shape")]
mod shape;
#[cfg(feature = "text")]
mod text;
#[cfg(feature = "window")]
mod window;

pub use {
    entity::{
        koto_entity_channel, KotoEntity, KotoEntityEvent, KotoEntityMapping, KotoEntityPlugin,
        KotoEntityReceiver, KotoEntitySender, UpdateKotoEntity,
    },
    runtime::{
        koto_channel, KotoReceiver, KotoRuntime, KotoRuntimePlugin, KotoSchedule, KotoScript,
        KotoSender, KotoUpdate, LoadScript, ScriptLoaded,
    },
};

#[cfg(feature = "camera")]
pub use camera::{KotoCamera, KotoCameraPlugin, UpdateOrthographicProjection};

#[cfg(feature = "color")]
pub use color::{
    koto_to_bevy_color, KotoColor, KotoColorPlugin, SetClearColor, UpdateColorMaterial,
};

#[cfg(feature = "geometry")]
pub use geometry::{KotoGeometryPlugin, KotoVec2, UpdateTransform};

#[cfg(feature = "random")]
pub use random::KotoRandomPlugin;

#[cfg(feature = "shape")]
pub use shape::KotoShapePlugin;

#[cfg(feature = "text")]
pub use text::KotoTextPlugin;

#[cfg(feature = "window")]
pub use window::KotoWindowPlugin;

pub use koto;

#[cfg(feature = "color")]
pub use koto_color;

#[cfg(feature = "geometry")]
pub use koto_geometry;

#[cfg(feature = "random")]
pub use koto_random;
