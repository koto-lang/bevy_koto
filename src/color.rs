//! Support for working with Bevy colors in Koto scripts

use crate::prelude::*;
use bevy::prelude::*;
use cloned::cloned;
use koto::prelude::*;
pub use koto_color::Color as KotoColor;

/// Color support for bevy_koto
///
/// The plugin adds the `color` module from `koto_color` to Koto's prelude,
/// along with a `set_clear_color` function.
#[derive(Default)]
pub struct KotoColorPlugin;

impl Plugin for KotoColorPlugin {
    fn build(&self, app: &mut App) {
        assert!(app.is_plugin_added::<KotoRuntimePlugin>());
        assert!(app.is_plugin_added::<KotoEntityPlugin>());

        let (set_clear_color_sender, set_clear_color_receiver) = koto_channel::<SetClearColor>();
        let (update_color_sender, update_color_receiver) =
            koto_entity_channel::<UpdateColorMaterial>();

        app.insert_resource(set_clear_color_sender)
            .insert_resource(set_clear_color_receiver)
            .insert_resource(update_color_sender)
            .insert_resource(update_color_receiver)
            .add_event::<SetClearColor>()
            .add_systems(Startup, on_startup)
            .add_systems(KotoSchedule, on_script_loaded.in_set(KotoUpdate::PreUpdate))
            .add_systems(
                Update,
                (set_clear_color, koto_to_bevy_color_material_events),
            );
    }
}

fn on_startup(koto: Res<KotoRuntime>, set_clear_color: Res<KotoSender<SetClearColor>>) {
    let prelude = koto.prelude();

    prelude.insert("color", koto_color::make_module());

    prelude.add_fn("set_clear_color", {
        cloned!(set_clear_color);
        move |ctx| {
            use KValue::*;

            let color = match ctx.args() {
                [Number(n1), Number(n2), Number(n3)] => {
                    Color::srgba(f32::from(n1), f32::from(n2), f32::from(n3), 1.0)
                }
                [Number(n1), Number(n2), Number(n3), Number(n4)] => {
                    Color::srgba(f32::from(n1), f32::from(n2), f32::from(n3), f32::from(n4))
                }
                [Object(o)] if o.is_a::<KotoColor>() => {
                    koto_to_bevy_color(&*o.cast::<KotoColor>()?)
                }
                unexpected => return unexpected_args("three or four Numbers", unexpected),
            };

            set_clear_color.send(SetClearColor(color));

            Ok(Null)
        }
    });
}

// Reset the clear color when a script is loaded
fn on_script_loaded(
    mut script_loaded_events: EventReader<ScriptLoaded>,
    mut clear_color: ResMut<ClearColor>,
) {
    for _ in script_loaded_events.read() {
        clear_color.0 = Color::BLACK;
    }
}

fn set_clear_color(channel: Res<KotoReceiver<SetClearColor>>, mut clear_color: ResMut<ClearColor>) {
    while let Some(event) = channel.receive() {
        clear_color.0 = event.0;
    }
}

/// Event sent to set the value of the ClearColor resource
#[derive(Clone, Event)]
pub struct SetClearColor(Color);

/// A function that converts a Koto color into a Bevy color
pub fn koto_to_bevy_color(koto_color: &KotoColor) -> Color {
    match koto_color.color {
        koto_color::Encoding::Srgb(c) => Color::srgba(c.red, c.green, c.blue, koto_color.alpha),
        koto_color::Encoding::Hsl(c) => {
            Color::hsla(c.hue.into(), c.saturation, c.lightness, koto_color.alpha)
        }
        koto_color::Encoding::Hsv(c) => {
            Color::hsva(c.hue.into(), c.saturation, c.value, koto_color.alpha)
        }
        koto_color::Encoding::Oklab(c) => Color::oklaba(c.l, c.a, c.b, koto_color.alpha),
        koto_color::Encoding::Oklch(c) => {
            Color::oklcha(c.l, c.chroma, c.hue.into(), koto_color.alpha)
        }
    }
}

fn koto_to_bevy_color_material_events(
    channel: Res<KotoEntityReceiver<UpdateColorMaterial>>,
    query: Query<&MeshMaterial2d<ColorMaterial>>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    while let Some(event) = channel.receive() {
        let handle = query.get(event.entity.get()).unwrap();
        let material = materials.get_mut(handle.id()).unwrap();
        match event.event {
            UpdateColorMaterial::Color(color) => material.color = color,
            UpdateColorMaterial::Alpha(alpha) => {
                material.color.set_alpha(alpha);
            }
            UpdateColorMaterial::SetImagePath(image_path) => {
                material.texture = image_path.map(|path| asset_server.load(path));
            }
        }
    }
}

/// Event for updating properties of a `ColorMaterial`
#[derive(Clone, Event)]
pub enum UpdateColorMaterial {
    /// Sets the material's color
    Color(Color),
    /// Sets the material's alpha value
    Alpha(f32),
    /// Sets the material's image path
    SetImagePath(Option<String>),
}
