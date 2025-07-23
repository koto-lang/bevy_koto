//! Support for adding and updating 2D shapes in Koto scripts

use crate::prelude::*;
use bevy::{prelude::*, render::view::RenderLayers};
use cloned::cloned;
use koto::{derive::*, prelude::*, runtime::Result as KotoResult};

/// Basic 2d shapes for bevy_koto
///
/// The plugin adds a `shape` module to the Koto prelude.
/// The currently available shapes are `circle`, `square`, and `polygon`.
#[derive(Default)]
pub struct KotoShapePlugin;

impl Plugin for KotoShapePlugin {
    fn build(&self, app: &mut App) {
        assert!(app.is_plugin_added::<KotoRuntimePlugin>());
        assert!(app.is_plugin_added::<KotoEntityPlugin>());
        assert!(app.is_plugin_added::<KotoColorPlugin>());
        assert!(app.is_plugin_added::<KotoGeometryPlugin>());

        let (spawn_shape_sender, spawn_shape_receiver) = koto_channel::<SpawnShape>();

        app.insert_resource(spawn_shape_sender)
            .insert_resource(spawn_shape_receiver)
            .add_systems(Startup, on_startup)
            .add_systems(KotoSchedule, spawn_shapes.in_set(KotoUpdate::PostUpdate));
    }
}

fn on_startup(
    koto: ResMut<KotoRuntime>,
    spawn_shape: Res<KotoSender<SpawnShape>>,
    update_shape: Res<KotoEntitySender<UpdateColorMaterial>>,
    update_entity: Res<KotoEntitySender<UpdateKotoEntity>>,
    update_transform: Res<KotoEntitySender<UpdateTransform>>,
) {
    let shape_module = KMap::with_type("shape");

    let make_shape = {
        cloned!(spawn_shape, update_entity, update_shape, update_transform);

        move |shape: Shape| {
            let entity = KotoEntityMapping::default();

            let result: KObject = KotoShape {
                entity: entity.clone(),
                state: KValue::Null,
                update_shape: update_shape.clone(),
                update_entity: update_entity.clone(),
                update_transform: update_transform.clone(),
            }
            .into();

            spawn_shape.send(SpawnShape {
                koto_entity: KotoEntity::new(result.clone(), entity),
                shape,
            });
            Ok(result.into())
        }
    };

    shape_module.add_fn("circle", {
        cloned!(make_shape);
        move |ctx| match ctx.args() {
            &[] => make_shape(Shape::Circle),
            unexpected => unexpected_args("no arguments", unexpected),
        }
    });

    shape_module.add_fn("polygon", {
        cloned!(make_shape);
        move |ctx| match ctx.args() {
            &[KValue::Number(n)] if n > 1 => make_shape(Shape::Polygon(n.into())),
            unexpected => unexpected_args("no arguments", unexpected),
        }
    });

    shape_module.add_fn("square", {
        cloned!(make_shape);
        move |ctx| match ctx.args() {
            &[] => make_shape(Shape::Rect(1.0, 1.0)),
            unexpected => unexpected_args("no arguments", unexpected),
        }
    });

    koto.prelude().insert("shape", shape_module);
}

fn spawn_shapes(
    channel: Res<KotoReceiver<SpawnShape>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
) {
    while let Some(SpawnShape {
        mut koto_entity,
        shape,
    }) = channel.receive()
    {
        let mesh: Mesh = match shape {
            Shape::Rect(width, height) => Rectangle::new(width, height).into(),
            Shape::Circle => Circle::default().into(),
            Shape::Polygon(sides) => RegularPolygon::new(1.0, sides).into(),
        };

        let bevy_entity = commands
            .spawn((
                Mesh2d(meshes.add(mesh)),
                MeshMaterial2d(materials.add(ColorMaterial {
                    color: Color::WHITE,
                    alpha_mode: bevy::sprite::AlphaMode2d::Blend,
                    uv_transform: default(),
                    texture: None,
                })),
                RenderLayers::layer(0),
                koto_entity.clone(),
            ))
            .id();
        koto_entity.entity.assign_bevy_entity(bevy_entity);
    }
}

#[derive(Clone, Debug)]
struct SpawnShape {
    koto_entity: KotoEntity,
    shape: Shape,
}

#[derive(Clone, Debug)]
enum Shape {
    Rect(f32, f32),
    Circle,
    Polygon(u32),
}

#[derive(Clone, KotoType, KotoCopy)]
#[koto(type_name = "Shape")]
struct KotoShape {
    entity: KotoEntityMapping,
    state: KValue,
    update_shape: KotoEntitySender<UpdateColorMaterial>,
    update_entity: KotoEntitySender<UpdateKotoEntity>,
    update_transform: KotoEntitySender<UpdateTransform>,
}

impl KotoObject for KotoShape {}

#[koto_impl]
impl KotoShape {
    #[koto_method]
    fn state(&self) -> KValue {
        self.state.clone()
    }

    #[koto_method]
    fn set_state(ctx: MethodContext<Self>) -> KotoResult<KValue> {
        match ctx.args {
            [state] => ctx.instance_mut()?.state = state.clone(),
            _ => return runtime_error!("Shape.set_state: Expected a single value"),
        };

        ctx.instance_result()
    }

    #[koto_method]
    fn set_alpha(ctx: MethodContext<Self>) -> KotoResult<KValue> {
        let alpha = match ctx.args {
            [KValue::Number(n)] => n.into(),
            _ => return runtime_error!("Shape.set_alpha: Expected a number"),
        };

        let this = ctx.instance()?;
        this.update_shape.send(KotoEntityEvent::new(
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
            [Object(o)] if o.is_a::<KotoColor>() => koto_to_bevy_color(&*o.cast::<KotoColor>()?),
            _ => {
                return runtime_error!("Shape.set_color: Expected a Color, or 3 or 4 numbers");
            }
        };

        let this = ctx.instance()?;
        this.update_shape.send(KotoEntityEvent::new(
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
        this.update_shape.send(KotoEntityEvent::new(
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

    #[koto_method]
    fn on_update(ctx: MethodContext<Self>) -> KotoResult<KValue> {
        let f = match ctx.args {
            [f] if f.is_callable() => f.clone(),
            _ => return runtime_error!("Shape.on_update: Expected a callable value"),
        };

        let this = ctx.instance()?;
        this.update_entity.send(KotoEntityEvent::new(
            this.entity.clone(),
            UpdateKotoEntity::SetOnUpdate(Some((f, ctx.vm.spawn_shared_vm()))),
        ));

        ctx.instance_result()
    }

    #[koto_method]
    fn despawn(ctx: MethodContext<Self>) -> KotoResult<KValue> {
        let this = ctx.instance()?;
        this.update_entity.send(KotoEntityEvent::new(
            this.entity.clone(),
            UpdateKotoEntity::Despawn,
        ));

        Ok(KValue::Null)
    }
}

impl From<KotoShape> for KValue {
    fn from(shape: KotoShape) -> Self {
        KObject::from(shape).into()
    }
}
