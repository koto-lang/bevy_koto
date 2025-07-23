//! A collection of useful items to import when using `bevy_koto`

pub use crate::entity::{
    koto_entity_channel, KotoEntity, KotoEntityEvent, KotoEntityMapping, KotoEntityPlugin,
    KotoEntityReceiver, KotoEntitySender, UpdateKotoEntity,
};
pub use crate::koto_plugins::KotoPlugins;
pub use crate::runtime::{
    koto_channel, KotoReceiver, KotoRuntime, KotoRuntimePlugin, KotoSchedule, KotoScript,
    KotoSender, KotoTime, KotoUpdate, LoadScript, ScriptLoaded,
};

#[cfg(feature = "camera")]
pub use crate::camera::{KotoCamera, KotoCameraPlugin, UpdateOrthographicProjection};

#[cfg(feature = "color")]
pub use crate::color::{
    koto_to_bevy_color, KotoColor, KotoColorPlugin, SetClearColor, UpdateColorMaterial,
};

#[cfg(feature = "geometry")]
pub use crate::geometry::{KotoGeometryPlugin, KotoVec2, UpdateTransform};

#[cfg(feature = "random")]
pub use crate::random::KotoRandomPlugin;

#[cfg(feature = "shape")]
pub use crate::shape::KotoShapePlugin;

#[cfg(feature = "text")]
pub use crate::text::KotoTextPlugin;

#[cfg(feature = "window")]
pub use crate::window::KotoWindowPlugin;
