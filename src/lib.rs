//! # bevy_koto
//!
//! Plugins for Bevy that add support for scripting with Koto.

#![warn(missing_docs)]

pub mod entity;
pub mod koto_plugins;
pub mod prelude;
pub mod runtime;

#[cfg(feature = "camera")]
pub mod camera;
#[cfg(feature = "color")]
pub mod color;
#[cfg(feature = "geometry")]
pub mod geometry;
#[cfg(feature = "random")]
pub mod random;
#[cfg(feature = "shape")]
pub mod shape;
#[cfg(feature = "text")]
pub mod text;
#[cfg(feature = "window")]
pub mod window;

pub use koto;

#[cfg(feature = "color")]
pub use koto_color;

#[cfg(feature = "geometry")]
pub use koto_geometry;

#[cfg(feature = "random")]
pub use koto_random;
