#![allow(clippy::needless_doctest_main)]

/// input settings for the editor UI
#[cfg(feature = "default_windows")]
pub mod controls;

use bevy::{
    prelude::{Entity, Plugin, Update},
    text::cosmic_text::Command,
    transform::commands,
    window::{MonitorSelection, Window, WindowPosition, WindowRef, WindowResolution},
};

use bevy_editor_pls_core::editor::EditorTabs;
pub use bevy_editor_pls_core::egui_dock;
#[doc(inline)]
pub use bevy_editor_pls_core::{editor, editor_window, AddEditorWindow};
use bevy_editor_pls_default_windows::{
    assets::AssetsWindow,
    logging::LoggingWindow,
    prelude::HierarchyWindow,
};
pub use egui;

#[cfg(feature = "default_windows")]
#[doc(inline)]
pub use bevy_editor_pls_default_windows as default_windows;

/// Commonly used types and extension traits
pub mod prelude {
    pub use crate::{AddEditorWindow, EditorPlugin};
    // #[cfg(feature = "default_windows")]
    // pub use bevy_editor_pls_default_windows::scenes::NotInScene;
}

/// Where to show the editor
#[derive(Default)]
pub enum EditorWindowPlacement {
    /// On the primary window
    #[default]
    Primary,
    /// Spawn a new window for the editor
    New(Window),
    /// On an existing window
    Window(Entity),
}

/// Plugin adding various editor UI to the game executable.
///
/// ```rust,no_run
/// use bevy::prelude::*;
/// use bevy_editor_pls::EditorPlugin;
///
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_plugins(EditorPlugin::new())
///         .run();
/// }
/// ```
#[derive(Default)]
pub struct EditorPlugin {
    pub window: EditorWindowPlacement,
}

impl EditorPlugin {
    pub fn new() -> Self {
        EditorPlugin::default()
    }

    /// Start the editor in a new window. Use [`Window::default`] for creating a new window with default settings.
    pub fn in_new_window(mut self, window: Window) -> Self {
        self.window = EditorWindowPlacement::New(window);
        self
    }
    /// Start the editor on the second window ([`MonitorSelection::Index(1)`].
    pub fn on_second_monitor_fullscreen(self) -> Self {
        self.in_new_window(Window {
            // TODO: just use `mode: BorderlessFullscreen` https://github.com/bevyengine/bevy/pull/8178
            resolution: WindowResolution::new(1920.0, 1080.0),
            position: WindowPosition::Centered(MonitorSelection::Index(1)),
            decorations: false,
            ..Default::default()
        })
    }
}

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let window = match self.window {
            EditorWindowPlacement::New(ref window) => {
                let mut window = window.clone();
                if window.title == "Bevy App" {
                    window.title = "bevy_editor_pls".into();
                }
                let entity = app.world_mut().spawn(window);
                WindowRef::Entity(entity.id())
            }
            EditorWindowPlacement::Window(entity) => WindowRef::Entity(entity),
            EditorWindowPlacement::Primary => WindowRef::Primary,
        };

        app.add_plugins(bevy_editor_pls_core::EditorPlugin { window });

        app.add_editor_window::<crate::controls::ControlsWindow>();
        app.insert_resource(controls::EditorControls::default_bindings())
            .add_systems(Update, controls::editor_controls_system);

        // if !app.is_plugin_added::<bevy_framepace::FramepacePlugin>() {
        //     app.add_plugins(bevy_framepace::FramepacePlugin);
        //     app.add_plugins(bevy_framepace::debug::DiagnosticsPlugin);
        // }

        use bevy_editor_pls_default_windows::prelude::*;
        app.add_plugins(HierarchyWindow);
        app.add_plugins(AssetsWindow);
        app.add_plugins(InspectorWindow);
        app.add_plugins(DebugSettingsWindow);
        // app.add_plugins(AddWindow);
        app.add_plugins(DiagnosticsWindow);
        app.add_plugins(RendererWindow);
        app.add_plugins(CameraWindow::default()); //TODO rework this, either with CameraWindowPlugin or by moving target camera into different component
        app.add_plugins(ResourcesWindow);
        // app.add_plugins(SceneWindow);
        app.add_plugins(MetricsWindow);
        app.add_plugins(LoggingWindow);
        app.add_plugins(SystemGraphWindow::default());
        app.add_plugins(GizmosWindow);
    }
}

use bevy::prelude::*;
#[cfg(feature = "default_windows")]
pub fn spawn_default_windows(mut commands: Commands, mut tree: ResMut<EditorTabs>) {
    use bevy_editor_pls_core::{editor_window::EditorWindowsCollection};
    use bevy_editor_pls_default_windows::prelude::*;
    let parent = commands
        .spawn((Name::new("Editor Windows"), EditorWindowsCollection))
        .id();

    let h = commands.spawn(HierarchyWindow).set_parent(parent).id();
    let r = commands.spawn(ResourcesWindow).set_parent(parent).id();
    let a = commands.spawn(AssetsWindow).set_parent(parent).id();
    let i = commands.spawn(InspectorWindow).set_parent(parent).id();

    let d1 = commands.spawn(DebugSettingsWindow).set_parent(parent).id();
    let d2 = commands.spawn(DiagnosticsWindow).set_parent(parent).id();

    let c = commands
        .spawn((
            bevy_editor_pls_default_windows::cameras::default_editor_cam(),
            CameraWindow::default(),
        ))
        .id();
    let c2 = commands.spawn(CameraWindow::default()).id();

    tree.state.push_to_first_leaf(h.into());
    tree.state.push_to_first_leaf(r.into());
    tree.state.push_to_first_leaf(a.into());

    // I am not a fan of egui_dock
    // this is just to focus on the hierarchy
    if let egui_dock::Node::Leaf { active, .. } = &mut tree.state.main_surface_mut()[0.into()] {
        *active = 0usize.into();
    }

    let [left, right] = tree.state.split(
        (0.into(), 0.into()),
        egui_dock::Split::Right,
        0.25,
        egui_dock::Node::leaf_with(vec![c.into(), c2.into()]),
    );

    if let egui_dock::Node::Leaf { active, .. } = &mut tree.state.main_surface_mut()[right] {
        // focus on editor camera
        *active = 0usize.into();
    }

    let [_, bottom] = tree.state.split(
        (0.into(), right),
        egui_dock::Split::Below,
        0.8,
        egui_dock::Node::leaf_with(vec![d1.into(), d2.into()]),
    );

    if let egui_dock::Node::Leaf { collapsed, .. } = &mut tree.state.main_surface_mut()[bottom] {
        // collapse bottom panel by default
        *collapsed = true;
    }

    tree.state.split(
        (0.into(), left),
        egui_dock::Split::Below,
        0.6,
        egui_dock::Node::leaf(i.into()),
    );

    info!("created default windows");
}
