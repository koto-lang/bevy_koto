use crate::{
    color::{
        koto_to_bevy_color, KotoColor, KotoColorPlugin, UpdateColorMaterial,
        UpdateColorMaterialSender,
    },
    entity::{
        KotoEntity, KotoEntityEvent, KotoEntityMapping, KotoEntityPlugin, UpdateEntity,
        UpdateEntitySender,
    },
    geometry::{KotoGeometryPlugin, KotoVec2, UpdateTransform, UpdateTransformSender},
    runtime::{
        make_channel, KotoReceiver, KotoRuntime, KotoRuntimePlugin, KotoSchedule, KotoSender,
        KotoUpdate,
    },
};
use bevy::prelude::*;
use cloned::cloned;
use koto::{derive::*, prelude::*, runtime::Result as KotoResult};

/// Text support for bevy_koto
///
/// This plugin is currently underbaked (appropriate font sizing needs to be figured out),
/// but it's a start.
pub struct KotoTextPlugin;

impl Plugin for KotoTextPlugin {
    fn build(&self, app: &mut App) {
        assert!(app.is_plugin_added::<KotoRuntimePlugin>());
        assert!(app.is_plugin_added::<KotoEntityPlugin>());
        assert!(app.is_plugin_added::<KotoColorPlugin>());
        assert!(app.is_plugin_added::<KotoGeometryPlugin>());

        let (spawn_text_sender, spawn_text_receiver) = make_channel::<SpawnText>();

        app.insert_resource(spawn_text_sender)
            .insert_resource(spawn_text_receiver)
            .add_systems(Startup, on_startup)
            .add_systems(KotoSchedule, spawn_text.in_set(KotoUpdate::PostUpdate));
    }
}

fn on_startup(
    koto: ResMut<KotoRuntime>,
    spawn_text: Res<SpawnTextSender>,
    update_material: Res<UpdateColorMaterialSender>,
    update_entity: Res<UpdateEntitySender>,
    update_transform: Res<UpdateTransformSender>,
) {
    let prelude = koto.prelude();
    prelude.add_fn("make_text", {
        cloned!(spawn_text, update_entity, update_material, update_transform);

        move |ctx| {
            let entity = KotoEntityMapping::new();

            let text = match ctx.args() {
                [KValue::Str(s)] => s.to_string(),
                [] => String::new(),
                unexpected => return unexpected_args("an optional string", unexpected),
            };

            let result: KObject = KotoText {
                entity: entity.clone(),
                update_material: update_material.clone(),
                update_entity: update_entity.clone(),
                update_transform: update_transform.clone(),
            }
            .into();

            spawn_text.send(SpawnText {
                koto_entity: KotoEntity::new(result.clone(), entity),
                text,
            });

            Ok(result.into())
        }
    });
}

fn spawn_text(channel: Res<SpawnTextReceiver>, mut commands: Commands) {
    while let Some(SpawnText {
        mut koto_entity,
        text,
    }) = channel.receive()
    {
        debug!("Spawning text '{text}'");
        let bevy_entity = commands
            .spawn((
                Text2dBundle {
                    text: Text::from_section(
                        text,
                        TextStyle {
                            font_size: 100.0,
                            ..Default::default()
                        },
                    )
                    .with_justify(JustifyText::Center),
                    ..default()
                },
                koto_entity.clone(),
            ))
            .id();
        koto_entity.entity.assign_bevy_entity(bevy_entity);
    }
}

#[derive(Clone, Debug)]
struct SpawnText {
    koto_entity: KotoEntity,
    text: String,
}

type SpawnTextSender = KotoSender<SpawnText>;
type SpawnTextReceiver = KotoReceiver<SpawnText>;

#[derive(Clone, KotoType, KotoCopy)]
#[koto(type_name = "Text")]
struct KotoText {
    entity: KotoEntityMapping,
    update_material: UpdateColorMaterialSender,
    update_entity: UpdateEntitySender,
    update_transform: UpdateTransformSender,
}

impl KotoObject for KotoText {}

#[koto_impl]
impl KotoText {
    #[koto_method]
    fn set_alpha(ctx: MethodContext<Self>) -> KotoResult<KValue> {
        let alpha = match ctx.args {
            [KValue::Number(n)] => n.into(),
            _ => return runtime_error!("Shape.set_alpha: Expected a number"),
        };

        let this = ctx.instance()?;
        this.update_material.send(KotoEntityEvent::new(
            this.entity.clone(),
            UpdateColorMaterial::Alpha(alpha),
        ));

        ctx.instance_result()
    }

    #[koto_method(alias = "set_colour")]
    fn set_color(ctx: MethodContext<Self>) -> KotoResult<KValue> {
        use KValue::{Number, Object};

        let color = match ctx.args {
            [Number(n1), Number(n2), Number(n3)] => {
                Color::srgba(f32::from(n1), f32::from(n2), f32::from(n3), 1.0)
            }
            [Number(n1), Number(n2), Number(n3), Number(n4)] => {
                Color::srgba(f32::from(n1), f32::from(n2), f32::from(n3), f32::from(n4))
            }
            [Object(o)] if o.is_a::<KotoColor>() => koto_to_bevy_color(*o.cast::<KotoColor>()?),
            _ => {
                return runtime_error!("Shape.set_color: Expected a Color, or 3 or 4 numbers");
            }
        };

        let this = ctx.instance()?;
        this.update_material.send(KotoEntityEvent::new(
            this.entity.clone(),
            UpdateColorMaterial::Color(color),
        ));

        ctx.instance_result()
    }

    #[koto_method]
    fn set_image(ctx: MethodContext<Self>) -> KotoResult<KValue> {
        let path = match ctx.args {
            [KValue::Str(path)] => path,
            _ => {
                return runtime_error!("Shape.set_image: Expected an image path as a string");
            }
        };

        let this = ctx.instance()?;
        this.update_material.send(KotoEntityEvent::new(
            this.entity.clone(),
            UpdateColorMaterial::SetImagePath(Some(path.to_string())),
        ));

        ctx.instance_result()
    }

    #[koto_method]
    fn set_position(ctx: MethodContext<Self>) -> KotoResult<KValue> {
        use KValue::{Number, Object};

        let position = match ctx.args {
            [Number(x), Number(y)] => Vec3::new(x.into(), y.into(), 0.0),
            [Number(x), Number(y), Number(z)] => Vec3::new(x.into(), y.into(), z.into()),
            [Object(v)] if v.is_a::<KotoVec2>() => {
                let v = v.cast::<KotoVec2>()?.inner();
                Vec3::new(v.x as f32, v.y as f32, 0.0)
            }
            [Object(v), Number(z)] if v.is_a::<KotoVec2>() => {
                let v = v.cast::<KotoVec2>()?.inner();
                Vec3::new(v.x as f32, v.y as f32, z.into())
            }
            _ => {
                return runtime_error!(
                    "Shape.set_position: Expected x, y, (and optionally z) positions"
                )
            }
        };

        let this = ctx.instance()?;
        this.update_transform
            .send(KotoEntityEvent::<UpdateTransform>::new(
                this.entity.clone(),
                UpdateTransform::Position(position),
            ));

        ctx.instance_result()
    }

    #[koto_method]
    fn set_rotation(ctx: MethodContext<Self>) -> KotoResult<KValue> {
        let rotation = match ctx.args {
            [KValue::Number(x)] => x.into(),
            _ => return runtime_error!("Shape.set_rotation: Expected a Number in radians"),
        };

        let this = ctx.instance()?;
        this.update_transform.send(KotoEntityEvent::new(
            this.entity.clone(),
            UpdateTransform::Rotation(rotation),
        ));

        ctx.instance_result()
    }

    #[koto_method]
    fn set_size(ctx: MethodContext<Self>) -> KotoResult<KValue> {
        use KValue::Number;

        let size = match ctx.args {
            [Number(size)] => {
                let size = f32::from(size);
                Vec3::new(size, size, 0.0)
            }
            [Number(x), Number(y)] => Vec3::new(f32::from(x), f32::from(y), 0.0),
            _ => return runtime_error!("Shape.set_size: Expected Numbers"),
        };

        let this = ctx.instance()?;
        this.update_transform.send(KotoEntityEvent::new(
            this.entity.clone(),
            UpdateTransform::Scale(size),
        ));

        ctx.instance_result()
    }

    // #[koto_method]
    // fn set_text(ctx: MethodContext<Self>) -> KotoResult<KValue> {
    //     let text = match ctx.args {
    //         [KValue::Str(text)] => Vec3::new(size, size, 0.0),
    //         _ => return runtime_error!("Text.set_text: Expected a string"),
    //     };
    //
    //     let this = ctx.instance()?;
    //     this.update_transform.send(KotoEntityEvent::new(
    //         this.entity.clone(),
    //         UpdateTransform::Scale(size),
    //     ));
    //
    //     ctx.instance_result()
    // }

    #[koto_method]
    fn on_update(ctx: MethodContext<Self>) -> KotoResult<KValue> {
        let f = match ctx.args {
            [f] if f.is_callable() => f.clone(),
            _ => return runtime_error!("Shape.on_update: Expected a callable value"),
        };

        let this = ctx.instance()?;
        this.update_entity.send(KotoEntityEvent::new(
            this.entity.clone(),
            UpdateEntity::SetOnUpdate(Some((f, ctx.vm.spawn_shared_vm()))),
        ));

        ctx.instance_result()
    }

    #[koto_method]
    fn despawn(ctx: MethodContext<Self>) -> KotoResult<KValue> {
        let this = ctx.instance()?;
        this.update_entity.send(KotoEntityEvent::new(
            this.entity.clone(),
            UpdateEntity::Despawn,
        ));

        Ok(KValue::Null)
    }
}

impl From<KotoText> for KValue {
    fn from(shape: KotoText) -> Self {
        KObject::from(shape).into()
    }
}
