//! See [`KotoPlugins`]

use crate::prelude::*;
use bevy::app::plugin_group;

plugin_group! {
    /// A group containing all available `bevy_koto` plugins
    pub struct KotoPlugins {
        :KotoRuntimePlugin,
        :KotoEntityPlugin,

        #[cfg(feature = "camera")]
        :KotoCameraPlugin,
        #[cfg(feature = "color")]
        :KotoColorPlugin,
        #[cfg(feature = "geometry")]
        :KotoGeometryPlugin,
        #[cfg(feature = "random")]
        :KotoRandomPlugin,
        #[cfg(feature = "shape")]
        :KotoShapePlugin,
        #[cfg(feature = "text")]
        :KotoTextPlugin,
        #[cfg(feature = "window")]
        :KotoWindowPlugin,
    }
}
