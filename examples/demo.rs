use std::path::Path;

use anyhow::Result;
use bevy::{asset::LoadedFolder, diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use bevy_koto::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(version)]
struct Args {
    /// The width of the window
    #[arg(short = 'W', long, default_value_t = 800)]
    width: u32,

    /// The height of the window
    #[arg(short = 'H', long, default_value_t = 600)]
    height: u32,

    /// The name of the script to run on launch
    #[arg(value_name = "SCRIPT_NAME", default_value = "scrolling_squares")]
    script: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!(
        "
>> Welcome to the bevy_koto demo <<

Press tab to load the next script.
Press R to reload the current script.
"
    );

    App::new()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "bevy_koto".into(),
                        resolution: (args.width as f32, args.height as f32).into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .set(AssetPlugin {
                    watch_for_changes_override: Some(true),
                    ..Default::default()
                }),
            FrameTimeDiagnosticsPlugin,
        ))
        .add_plugins((
            KotoRuntimePlugin,
            KotoEntityPlugin,
            KotoCameraPlugin,
            KotoWindowPlugin,
            KotoColorPlugin,
            KotoGeometryPlugin,
            KotoRandomPlugin,
            KotoShapePlugin,
            KotoTextPlugin,
        ))
        .init_state::<AppState>()
        .add_systems(OnEnter(AppState::Setup), setup)
        .add_systems(
            Update,
            check_script_events.run_if(in_state(AppState::Setup)),
        )
        .add_systems(OnEnter(AppState::Ready), ready)
        .add_systems(Update, process_keypresses.run_if(in_state(AppState::Ready)))
        .run();

    Ok(())
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, States)]
enum AppState {
    #[default]
    Setup,
    Ready,
}

fn setup(asset_server: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(Camera2d::default()).insert(KotoCamera);

    commands.insert_resource(ScriptLoader {
        script_folder: asset_server.load_folder("scripts"),
        ..default()
    });
}

fn check_script_events(
    mut next_state: ResMut<NextState<AppState>>,
    script_loader: Res<ScriptLoader>,
    mut events: EventReader<AssetEvent<LoadedFolder>>,
) {
    for event in events.read() {
        if event.is_loaded_with_dependencies(&script_loader.script_folder) {
            next_state.set(AppState::Ready);
        }
    }
}

fn ready(
    loaded_folders: Res<Assets<LoadedFolder>>,
    mut script_loader: ResMut<ScriptLoader>,
    mut scripts: ResMut<Assets<KotoScript>>,
    mut load_script: EventWriter<LoadScript>,
) {
    let script_folder = loaded_folders
        .get(&script_loader.script_folder)
        .expect("Missing script folder");

    for handle in script_folder.handles.iter() {
        if let Ok(script_id) = handle.id().try_typed::<KotoScript>() {
            let Some(script) = scripts.get(script_id) else {
                error!("Script missing (id: {script_id})");
                continue;
            };

            // We only want to make top-level scripts available for loading
            let mut ancestors = script.path.ancestors();
            ancestors.next();
            if ancestors.next() == Some(Path::new("scripts"))
                && ancestors.next() == Some(Path::new(""))
            {
                info!("Loaded script: {}", script.path.to_string_lossy());

                let Some(script_handle) = scripts.get_strong_handle(script_id) else {
                    error!("Failed to get strong handle (id: {script_id})");
                    continue;
                };

                script_loader.scripts.push(script_handle);
            }
        }
    }

    script_loader
        .scripts
        .sort_by_key(|id| &scripts.get(id).unwrap().path);

    script_loader.next_script(&mut load_script);
}

fn process_keypresses(
    input: Res<ButtonInput<KeyCode>>,
    mut load_script_events: EventWriter<LoadScript>,
    mut script_loader: ResMut<ScriptLoader>,
) {
    if input.just_pressed(KeyCode::Tab) {
        if input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
            script_loader.previous_script(&mut load_script_events);
        } else {
            script_loader.next_script(&mut load_script_events);
        }
    } else if input.just_pressed(KeyCode::KeyR) {
        script_loader.reload_script(&mut load_script_events);
    }
}

#[derive(Resource, Default)]
struct ScriptLoader {
    script_folder: Handle<LoadedFolder>,
    scripts: Vec<Handle<KotoScript>>,
    current_script: Option<usize>,
}

impl ScriptLoader {
    fn next_script(&mut self, load_script_events: &mut EventWriter<LoadScript>) {
        let next_index = self
            .current_script
            .map_or(0, |index| (index + 1) % self.scripts.len());
        self.load_script(next_index, load_script_events);
    }

    fn previous_script(&mut self, load_script_events: &mut EventWriter<LoadScript>) {
        let previous_index = self.current_script.map_or(0, |index| {
            if index > 0 {
                index - 1
            } else {
                self.scripts.len().saturating_sub(1)
            }
        });
        self.load_script(previous_index, load_script_events);
    }

    fn reload_script(&mut self, load_script_events: &mut EventWriter<LoadScript>) {
        if let Some(index) = self.current_script {
            self.load_script(index, load_script_events);
        }
    }

    fn load_script(&mut self, index: usize, load_script_events: &mut EventWriter<LoadScript>) {
        if let Some(script) = self.scripts.get(index) {
            load_script_events.send(LoadScript::load(script.clone()));
            self.current_script = Some(index);
        }
    }
}
