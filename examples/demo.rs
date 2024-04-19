use anyhow::{Context, Result};
use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin, ecs::schedule::ExecutorKind, prelude::*,
    window::close_on_esc,
};
use bevy_koto::*;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version)]
struct Args {
    /// The width of the window
    #[arg(short = 'W', long, default_value_t = 800)]
    width: u32,

    /// The height of the window
    #[arg(short = 'H', long, default_value_t = 600)]
    height: u32,

    /// The path of the assets dir
    #[arg(short, long, value_name = "DIR", default_value = "assets")]
    assets_dir: PathBuf,

    /// The name of the script to run from the assets dir
    #[arg(value_name = "SCRIPT_NAME", default_value = "scrolling_squares")]
    script: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!(
        "Welcome to the bevy_koto demo!
  - Tab: load the next script
  - Esc: exit the application
"
    );

    let assets_dir = args
        .assets_dir
        .canonicalize()
        .context("failed to canonicalize assets dir")?;

    App::new()
        .edit_schedule(Main, |schedule| {
            schedule.set_executor_kind(ExecutorKind::MultiThreaded);
        })
        .insert_resource(KotoScriptFolder::new(&assets_dir, Some(&args.script)))
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
                    file_path: assets_dir.to_string_lossy().into(),
                    // Enable file watching in release builds
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
        .add_systems(Update, (close_on_esc, load_script))
        .run();

    Ok(())
}

fn load_script(
    input: Res<ButtonInput<KeyCode>>,
    mut scripts: ResMut<KotoScriptFolder>,
    mut reload_script: EventWriter<ReloadScript>,
) {
    if input.just_pressed(KeyCode::Tab) {
        if input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
            info!("Selecting previous script");
            scripts.previous_script();
        } else {
            info!("Selecting next script");
            scripts.next_script();
        }
    } else if input.just_pressed(KeyCode::KeyR) {
        info!("Reloading script");
        reload_script.send_default();
    }
}
