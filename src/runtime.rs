use bevy::{
    app::MainScheduleOrder,
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    ecs::schedule::ScheduleLabel,
    prelude::*,
    reflect::TypePath,
    utils::BoxedFuture,
};
use cloned::cloned;
use koto::prelude::*;
use std::{
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    str,
    time::Duration,
};

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct KotoSchedule;

/// KotoUpdate sets are executed during PreUpdate
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

            let mut order = app.world.resource_mut::<MainScheduleOrder>();
            order.insert_after(PreUpdate, KotoSchedule);
        }

        let (add_dependency_sender, add_dependency_receiver) = make_channel::<AddDependency>();
        let koto_runtime = KotoRuntime::new(add_dependency_sender.clone());

        app.insert_resource(koto_runtime)
            .insert_resource(add_dependency_sender)
            .insert_resource(add_dependency_receiver)
            .insert_resource(KotoScriptAssets::default())
            .add_event::<ReloadScript>()
            .add_event::<ScriptLoaded>()
            .add_event::<ScriptCompiled>()
            .init_asset::<KotoScript>()
            .register_asset_loader(KotoScriptAssetLoader)
            .add_systems(
                KotoSchedule,
                (
                    // Koto update stages
                    (
                        // Request the current script from the asset server
                        load_script,
                        // Compile the script if necessary
                        compile_script,
                    )
                        .in_set(KotoUpdate::Compile),
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

fn load_script(
    script_folder: Res<KotoScriptFolder>,
    asset_server: Res<AssetServer>,
    mut script_assets: ResMut<KotoScriptAssets>,
) {
    if script_folder.is_changed() {
        let script_name = script_folder.current_script_name();
        script_assets.script = asset_server.load(script_name);
        script_assets.dependencies.clear();
    }
}

fn process_script_asset_events(
    script_assets: Res<KotoScriptAssets>,
    mut asset_events: EventReader<AssetEvent<KotoScript>>,
    mut reload_script_events: EventWriter<ReloadScript>,
) {
    for event in asset_events.read() {
        match event {
            AssetEvent::Added { id } if *id == script_assets.script.id() => {
                reload_script_events.send(ReloadScript { call_setup: true });
            }
            AssetEvent::Modified { .. } => {
                reload_script_events.send(ReloadScript { call_setup: false });
            }
            _ => continue,
        }
    }
}

fn compile_script(
    script_folder: Res<KotoScriptFolder>,
    script_assets: Res<KotoScriptAssets>,
    assets: Res<Assets<KotoScript>>,
    mut reload_script_events: EventReader<ReloadScript>,
    mut script_loaded: EventWriter<ScriptLoaded>,
    mut script_compiled: EventWriter<ScriptCompiled>,
    mut koto: ResMut<KotoRuntime>,
) {
    let mut load_script = false;
    let mut call_setup = false;

    for event in reload_script_events.read() {
        load_script = true;
        call_setup |= event.call_setup;
    }

    if load_script {
        let script = assets.get(&script_assets.script).unwrap();
        let script_path_in_assets_folder = script_folder
            .path
            .join(script_assets.script.path().unwrap().path());

        if call_setup {
            debug!("Sending ScriptLoaded event");
            script_loaded.send_default();
        }

        if koto
            .compile_script(&script.0, script_path_in_assets_folder, call_setup)
            .is_ok()
        {
            script_compiled.send_default();
        }
    }
}

#[derive(Event, Default)]
pub struct ReloadScript {
    call_setup: bool,
}

#[derive(Event, Default)]
pub struct ScriptLoaded;

#[derive(Event, Default)]
pub struct ScriptCompiled;

fn run_script_update(mut koto: ResMut<KotoRuntime>, time: Res<Time>) {
    if koto.is_ready {
        koto.run_update(time.delta_seconds_f64());
    }
}

fn add_script_dependencies(
    asset_server: Res<AssetServer>,
    script_folder: Res<KotoScriptFolder>,
    channel: Res<AddDependencyReceiver>,
    mut script_assets: ResMut<KotoScriptAssets>,
) {
    while let Some(dependency) = channel.receive() {
        let handle = asset_server.load(
            dependency
                .0
                .strip_prefix(&script_folder.path)
                .unwrap()
                .to_path_buf(),
        );
        script_assets.dependencies.push(handle);
    }
}

// The folder that the Koto scripts are contained in
#[derive(Debug, Resource)]
pub struct KotoScriptFolder {
    path: PathBuf,
    script_paths: Vec<PathBuf>,
    current_script: usize,
}

impl KotoScriptFolder {
    pub fn new(path: &Path, initial_script: Option<&str>) -> Self {
        let koto_ext = OsStr::new("koto");
        let mut script_paths: Vec<PathBuf> = fs::read_dir(path)
            .unwrap()
            .filter_map(|dir_entry| {
                let entry_path = dir_entry.unwrap().path();
                if entry_path.extension() == Some(koto_ext) {
                    Some(entry_path)
                } else {
                    None
                }
            })
            .collect();

        script_paths.sort();

        let current_script = if let Some(initial_script) = initial_script {
            let script_name = if initial_script.ends_with(".koto") {
                OsString::from(initial_script)
            } else {
                OsString::from(format!("{initial_script}.koto"))
            };
            script_paths
                .iter()
                .position(|path| path.file_name() == Some(&script_name))
                .expect("Invalid script name")
        } else {
            0
        };

        Self {
            path: path.into(),
            script_paths,
            current_script,
        }
    }

    pub fn current_script_name(&self) -> String {
        self.script_paths[self.current_script]
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .into()
    }

    pub fn next_script(&mut self) {
        if self.current_script == self.script_paths.len() - 1 {
            self.current_script = 0;
        } else {
            self.current_script += 1;
        }
    }

    pub fn previous_script(&mut self) {
        if self.current_script == 0 {
            self.current_script = self.script_paths.len() - 1;
        } else {
            self.current_script -= 1;
        }
    }
}

// The script as loaded by the asset loader
#[derive(Asset, TypePath, Debug)]
pub struct KotoScript(Box<str>);

// The currently loaded script assets
#[derive(Default, Resource)]
struct KotoScriptAssets {
    script: Handle<KotoScript>,
    dependencies: Vec<Handle<KotoScript>>,
}

#[derive(Debug, thiserror::Error)]
pub enum KotoScriptAssetLoaderError {
    #[error("Failed to load script: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse script as UTF-8: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}

#[derive(Default)]
pub struct KotoScriptAssetLoader;

impl AssetLoader for KotoScriptAssetLoader {
    type Asset = KotoScript;
    type Settings = ();
    type Error = KotoScriptAssetLoaderError;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a (),
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let script = str::from_utf8(&bytes)?;
            Ok(KotoScript(script.into()))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["koto"]
    }
}

#[derive(Default, Resource)]
pub struct KotoRuntime {
    runtime: Koto,
    user_data: KValue,
    is_ready: bool,
}

impl KotoRuntime {
    fn new(add_dependency_sender: AddDependencySender) -> Self {
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

    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    fn compile_script(
        &mut self,
        script: &str,
        script_path: PathBuf,
        call_setup: bool,
    ) -> Result<(), ()> {
        let now = std::time::Instant::now();

        self.is_ready = false;

        if let Err(error) = self.runtime.set_script_path(Some(script_path)) {
            error!("Error while setting script path:\n{error}");
            return Err(());
        }

        self.runtime.clear_module_cache();
        if let Err(error) = self.runtime.compile(script) {
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

    pub fn prelude(&self) -> &KMap {
        self.runtime.prelude()
    }

    pub fn user_data(&self) -> &KValue {
        &self.user_data
    }
}

pub fn make_channel<T>() -> (KotoSender<T>, KotoReceiver<T>) {
    let (sender, receiver) = crossbeam_channel::unbounded();
    (KotoSender(sender), KotoReceiver(receiver))
}

#[derive(Clone, Debug, Resource)]
pub struct KotoSender<T>(crossbeam_channel::Sender<T>);

impl<T> KotoSender<T> {
    pub fn send(&self, value: T) {
        self.0.try_send(value).expect("Failed to send value")
    }
}

#[derive(Clone, Debug, Resource)]
pub struct KotoReceiver<T>(crossbeam_channel::Receiver<T>);

impl<T> KotoReceiver<T> {
    pub fn receive(&self) -> Option<T> {
        self.0.try_recv().ok()
    }
}

#[derive(Clone, Debug)]
struct AddDependency(PathBuf);

type AddDependencySender = KotoSender<AddDependency>;
type AddDependencyReceiver = KotoReceiver<AddDependency>;
