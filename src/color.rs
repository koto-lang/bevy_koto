use crate::{
    entity::{KotoEntityEvent, KotoEntityPlugin},
    runtime::{
        make_channel, KotoReceiver, KotoRuntime, KotoRuntimePlugin, KotoSchedule, KotoSender,
        KotoUpdate, ScriptLoaded,
    },
};
use bevy::prelude::*;
use cloned::cloned;
use koto::prelude::*;
pub use koto_color::Color as KotoColor;

pub struct KotoColorPlugin;

impl Plugin for KotoColorPlugin {
    fn build(&self, app: &mut App) {
        assert!(app.is_plugin_added::<KotoRuntimePlugin>());
        assert!(app.is_plugin_added::<KotoEntityPlugin>());

        let (set_clear_color_sender, set_clear_color_receiver) = make_channel::<SetClearColor>();
        let (update_color_sender, update_color_receiver) =
            make_channel::<UpdateColorMaterialEvent>();

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

fn on_startup(koto: Res<KotoRuntime>, set_clear_color: Res<SetClearColorSender>) {
    let prelude = koto.prelude();

    prelude.insert("color", koto_color::make_module());

    prelude.add_fn("set_clear_color", {
        cloned!(set_clear_color);
        move |ctx| {
            use KValue::*;

            let color = match ctx.args() {
                [Number(n1), Number(n2), Number(n3)] => {
                    Color::rgba(f32::from(n1), f32::from(n2), f32::from(n3), 1.0)
                }
                [Number(n1), Number(n2), Number(n3), Number(n4)] => {
                    Color::rgba(f32::from(n1), f32::from(n2), f32::from(n3), f32::from(n4))
                }
                [Object(o)] if o.is_a::<KotoColor>() => koto_to_bevy_color(*o.cast::<KotoColor>()?),
                unexpected => {
                    return type_error_with_slice("three or four Numbers", unexpected);
                }
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

fn set_clear_color(channel: Res<SetClearColorReceiver>, mut clear_color: ResMut<ClearColor>) {
    while let Some(event) = channel.receive() {
        clear_color.0 = event.0;
    }
}

#[derive(Clone, Event)]
pub struct SetClearColor(Color);

pub type SetClearColorSender = KotoSender<SetClearColor>;
type SetClearColorReceiver = KotoReceiver<SetClearColor>;

pub fn koto_to_bevy_color(c: KotoColor) -> Color {
    let c = c.inner();
    Color::rgba(c.red, c.green, c.blue, c.alpha)
}

fn koto_to_bevy_color_material_events(
    channel: Res<UpdateColorMaterialReceiver>,
    query: Query<&Handle<ColorMaterial>>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    while let Some(event) = channel.receive() {
        let handle = query.get(event.entity.get()).unwrap();
        let material = materials.get_mut(handle.id()).unwrap();
        match event.event {
            UpdateColorMaterial::Color(color) => material.color = color,
            UpdateColorMaterial::Alpha(alpha) => {
                material.color.set_a(alpha);
            }
            UpdateColorMaterial::SetImagePath(image_path) => {
                material.texture = image_path.map(|path| asset_server.load(path));
            }
        }
    }
}

#[derive(Clone, Event)]
pub enum UpdateColorMaterial {
    Color(Color),
    Alpha(f32),
    SetImagePath(Option<String>),
}

pub type UpdateColorMaterialEvent = KotoEntityEvent<UpdateColorMaterial>;
pub type UpdateColorMaterialSender = KotoSender<UpdateColorMaterialEvent>;
pub type UpdateColorMaterialReceiver = KotoReceiver<UpdateColorMaterialEvent>;
