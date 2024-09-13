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

pub use camera::KotoCameraPlugin;
pub use color::KotoColorPlugin;
pub use entity::KotoEntityPlugin;
pub use geometry::KotoGeometryPlugin;
pub use random::KotoRandomPlugin;
pub use runtime::{KotoRuntimePlugin, KotoScript, LoadScript};
pub use shape::KotoShapePlugin;
pub use text::KotoTextPlugin;
pub use window::KotoWindowPlugin;
