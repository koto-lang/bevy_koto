//! Support for adding a Koto runtime to a Bevy application

use bevy::{
    app::MainScheduleOrder,
    asset::{io::Reader, AssetLoader, LoadContext},
    ecs::schedule::ScheduleLabel,
    prelude::*,
    reflect::TypePath,
};
use cloned::cloned;
use koto::prelude::*;
use std::{
    path::{Path, PathBuf},
    str,
    time::Duration,
};

/// The schedule used to update the Koto runtime
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct KotoSchedule;

/// The system set used for updating the Koto runtime
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum KotoUpdate {
    /// Detect if the script needs to be reloaded and compiled
    /// - The script's setup function gets called when the script is first loaded.
    /// - The script's on_load function gets called after each compilation.
    Compile,
    /// Actions that should happen before the script's update functions are called
    /// e.g.
    ///   - Despawning no-longer-needed Bevy entities
    ///   - Responding to Bevy events like window size updates
    PreUpdate,
    /// The set where the script's update functions get called
    Update,
    /// After the script updates have occurred, prepare for the Bevy Update stage
    /// e.g.
    ///   - Spawn any new entities
    ///   - Track newly imported script dependencies
    PostUpdate,
}

/// Manages a Koto runtime for Bevy
///
/// The [KotoSchedule] schedule is set up by the plugin, with the [KotoUpdate] system sets.
///
/// The following events are also added by the plugin:
/// - [LoadScript]: Sent to load a new script
/// - [ScriptLoaded]: Sent after a script has been successfully loaded and initialized.
#[derive(Default)]
pub struct KotoRuntimePlugin;

impl Plugin for KotoRuntimePlugin {
    fn build(&self, app: &mut App) {
        {
            app.init_schedule(KotoSchedule).configure_sets(
                KotoSchedule,
                (
                    KotoUpdate::Compile,
                    KotoUpdate::PreUpdate,
                    KotoUpdate::Update,
                    KotoUpdate::PostUpdate,
                )
                    .chain(),
            );

            let mut order = app.world_mut().resource_mut::<MainScheduleOrder>();
            order.insert_after(PreUpdate, KotoSchedule);
        }

        let (add_dependency_sender, add_dependency_receiver) = koto_channel::<AddDependency>();
        let koto_runtime = KotoRuntime::new(add_dependency_sender.clone());

        let Some(assets_root_path) = app
            .get_added_plugins::<AssetPlugin>()
            .last()
            .map(|plugin| PathBuf::from(&plugin.file_path))
        else {
            error!("AssetPlugin must be initialized before KotoRuntimePlugin");
            return;
        };

        app.insert_resource(koto_runtime)
            .insert_resource(add_dependency_sender)
            .insert_resource(add_dependency_receiver)
            .insert_resource(ActiveScript::default())
            .insert_resource(AssetsRootPath(assets_root_path))
            .add_event::<LoadScript>()
            .add_event::<ScriptLoaded>()
            .init_asset::<KotoScript>()
            .register_asset_loader(KotoScriptAssetLoader)
            .add_systems(
                KotoSchedule,
                (
                    // Compile the script if necessary
                    process_load_script_events.in_set(KotoUpdate::Compile),
                    // Run the script's update function
                    run_script_update.in_set(KotoUpdate::Update),
                    // Post update tasks
                    add_script_dependencies.in_set(KotoUpdate::PostUpdate),
                ),
            )
            .add_systems(
                Update,
                (process_script_asset_events, add_script_dependencies),
            );
    }
}

fn process_script_asset_events(
    active_script: Res<ActiveScript>,
    mut asset_events: EventReader<AssetEvent<KotoScript>>,
    mut load_script: EventWriter<LoadScript>,
) {
    if let Some(script) = &active_script.script {
        for event in asset_events.read() {
            let id = match event {
                AssetEvent::Added { id } => *id,
                AssetEvent::Modified { id } => *id,
                _ => continue,
            };

            if id == script.id()
                || active_script
                    .dependencies
                    .iter()
                    .any(|handle| id == handle.id())
            {
                load_script.send(LoadScript::reload(script.clone()));
            }
        }
    }
}

fn process_load_script_events(
    assets_root_path: Res<AssetsRootPath>,
    assets: Res<Assets<KotoScript>>,
    mut load_script_events: EventReader<LoadScript>,
    mut script_loaded: EventWriter<ScriptLoaded>,
    mut koto: ResMut<KotoRuntime>,
    mut active_script: ResMut<ActiveScript>,
) {
    for event in load_script_events.read() {
        let Some(script) = assets.get(event.script.id()) else {
            error!("Unable to load script (id: {})", event.script.id());
            continue;
        };

        info!("Loading {}", script.path.to_string_lossy());

        let script_path = assets_root_path.0.join(&script.path);
        if koto
            .initialize_script(&script.script, Some(&script_path), event.call_setup)
            .is_ok()
        {
            if event.call_setup {
                script_loaded.send_default();
            }

            active_script.script = Some(event.script.clone());
            active_script.dependencies.clear();
        }
    }
}

/// Sending this event will load the provided script into the runtime
#[derive(Event, Default)]
pub struct LoadScript {
    script: Handle<KotoScript>,
    call_setup: bool, // false for a hot-reload
}

impl LoadScript {
    /// Creates a LoadScript event for the given script handle
    pub fn load(script: Handle<KotoScript>) -> Self {
        Self {
            script,
            call_setup: true,
        }
    }

    /// Creates a LoadScript event for the given handle that skips the script's setup function
    pub fn reload(script: Handle<KotoScript>) -> Self {
        Self {
            script,
            call_setup: false,
        }
    }
}

/// Sent when a script has been successfully compiled and initialized
///
/// An event isn't sent when a script has been reloaded while running
/// (i.e. when LoadScript::call_setup is false).
#[derive(Event, Default)]
pub struct ScriptLoaded;

fn run_script_update(mut koto: ResMut<KotoRuntime>, time: Res<Time>) {
    if koto.is_ready {
        koto.run_update(time.delta_secs_f64());
    }
}

fn add_script_dependencies(
    assets_root_path: Res<AssetsRootPath>,
    asset_server: Res<AssetServer>,
    channel: Res<KotoReceiver<AddDependency>>,
    mut active_script: ResMut<ActiveScript>,
) {
    while let Some(dependency) = channel.receive() {
        if let Ok(path_in_assets) = dependency.0.strip_prefix(&assets_root_path.0) {
            let handle = asset_server.load(path_in_assets.to_owned());
            active_script.dependencies.push(handle);
        } else {
            error!(
                "Unable to find path in assets for {}",
                dependency.0.to_string_lossy()
            );
        }
    }
}

/// A Koto script as read from the assets folder
#[derive(Asset, TypePath, Debug)]
pub struct KotoScript {
    /// The script's contents
    pub script: String,
    /// The script's path in the assets folder
    ///
    /// Note that Koto currently requires absolute paths for dependency resolution, so this path
    /// needs to be converted to include the asset folder's path before passing it to Koto.
    pub path: PathBuf,
}

// The currently loaded script assets
#[derive(Default, Resource)]
struct ActiveScript {
    script: Option<Handle<KotoScript>>,
    dependencies: Vec<Handle<KotoScript>>,
}

#[derive(Default, Resource)]
struct AssetsRootPath(PathBuf);

#[derive(Debug, thiserror::Error)]
enum KotoScriptAssetLoaderError {
    #[error("Failed to load script: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse script as UTF-8: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}

#[derive(Default)]
struct KotoScriptAssetLoader;

impl AssetLoader for KotoScriptAssetLoader {
    type Asset = KotoScript;
    type Settings = ();
    type Error = KotoScriptAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let script = str::from_utf8(&bytes)?.to_string();
        Ok(KotoScript {
            script,
            path: load_context.path().into(),
        })
    }

    fn extensions(&self) -> &[&str] {
        &["koto"]
    }
}

/// The Koto runtime
#[derive(Default, Resource)]
pub struct KotoRuntime {
    runtime: Koto,
    user_data: KValue,
    is_ready: bool,
}

impl KotoRuntime {
    fn new(add_dependency_sender: KotoSender<AddDependency>) -> Self {
        let runtime = Koto::with_settings(
            KotoSettings::default()
                .with_execution_limit(Duration::from_secs(1))
                .with_module_imported_callback({
                    cloned!(add_dependency_sender);
                    move |path| {
                        add_dependency_sender.send(AddDependency(path.to_owned()));
                    }
                }),
        );

        Self {
            runtime,
            user_data: KValue::Null,
            is_ready: false,
        }
    }

    /// Returns true if a script has been successfully loaded
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    fn initialize_script(
        &mut self,
        script: &str,
        script_path: Option<&Path>,
        call_setup: bool,
    ) -> Result<(), ()> {
        let now = std::time::Instant::now();

        self.is_ready = false;

        self.runtime.clear_module_cache();
        let compile_args = CompileArgs {
            script,
            script_path: script_path
                .and_then(|path| path.to_str())
                .map(|path| KString::from(path)),
            compiler_settings: default(),
        };
        if let Err(error) = self.runtime.compile(compile_args) {
            error!("Error while compiling script:\n{error}");
            return Err(());
        }

        if call_setup {
            self.runtime.exports_mut().clear();
        }

        if let Err(e) = self.runtime.run() {
            error!("Error while running Koto script:\n{e}");
            return Err(());
        }

        if call_setup {
            debug!("Calling setup");
            self.user_data = match self.run_exported_function("setup", &[]) {
                Ok(Some(data)) => data,
                Ok(None) => KMap::default().into(),
                Err(e) => {
                    error!("Error in 'setup':\n{e}");
                    return Err(());
                }
            };
        }

        debug!("Calling on_load");
        if let Err(e) = self.run_exported_function("on_load", &[self.user_data.clone()]) {
            error!("Error in 'on_load':\n{e}");
            return Err(());
        }

        self.is_ready = true;

        info!(
            "Script ready in {:.3}ms",
            now.elapsed().as_secs_f64() * 1000.0
        );

        Ok(())
    }

    fn run_update(&mut self, time_delta: f64) {
        debug_assert!(self.is_ready);

        let now = std::time::Instant::now();

        if let Err(e) =
            self.run_exported_function("update", &[self.user_data.clone(), time_delta.into()])
        {
            error!("Error in 'update':\n{e}");
            return;
        }

        trace!("update: {:.3}ms", now.elapsed().as_secs_f64() * 1000.0)
    }

    /// Runs a function that has been exported from the currently running script
    pub fn run_exported_function(
        &mut self,
        function_name: &str,
        args: &[KValue],
    ) -> Result<Option<KValue>, koto::Error> {
        let Some(function) = self.runtime.exports().data().get(function_name).cloned() else {
            return Ok(None);
        };

        match self.runtime.call_function(function, args) {
            Ok(result) => Ok(Some(result)),
            Err(error) => {
                self.is_ready = false;
                Err(error)
            }
        }
    }

    /// The Koto runtime's prelude
    pub fn prelude(&self) -> &KMap {
        self.runtime.prelude()
    }

    /// The user data that is being held by the current script
    pub fn user_data(&self) -> &KValue {
        &self.user_data
    }
}

/// A helper for making a channel for events from Koto -> Bevy
pub fn koto_channel<T>() -> (KotoSender<T>, KotoReceiver<T>) {
    let (sender, receiver) = crossbeam_channel::unbounded();
    (KotoSender(sender), KotoReceiver(receiver))
}

/// A sender for events from Koto -> Bevy
///
/// See [koto_channel]
#[derive(Debug, Resource)]
pub struct KotoSender<T>(pub crossbeam_channel::Sender<T>);

impl<T> KotoSender<T> {
    /// Sends a value on the channel
    ///
    /// This is non-blocking, and will panic if sending fails.
    pub fn send(&self, value: T) {
        self.0.try_send(value).expect("Failed to send value")
    }
}

impl<T> Clone for KotoSender<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// A receiver for events from Koto -> Bevy
///
/// See [koto_channel]
#[derive(Clone, Debug, Resource)]
pub struct KotoReceiver<T>(pub crossbeam_channel::Receiver<T>);

impl<T> KotoReceiver<T> {
    /// Receives a value on the channel
    ///
    /// This is non-blocking, if no value is available then `None` is returned.
    pub fn receive(&self) -> Option<T> {
        self.0.try_recv().ok()
    }
}

#[derive(Clone, Debug)]
struct AddDependency(PathBuf);
