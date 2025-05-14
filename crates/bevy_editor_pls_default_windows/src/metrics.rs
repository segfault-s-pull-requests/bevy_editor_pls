/// reimplements some of the top level stuff in bevy_metrics_dashboard to work with editor
use std::{any::type_name, ops::DerefMut};

use bevy_editor_pls_core::{
    editor_window::{EditorWindow, EditorWindowContext},
    AddEditorWindow,
};
use bevy::{prelude::*, ui};
use bevy_inspector_egui::egui::{self, Ui};
use bevy_metrics_dashboard::{
    registry::MetricsRegistry, CachedPlotConfigs, ClearBucketsSystem, CoreMetricsPlugin, DashboardPlugin, DashboardWindow, NamespaceTreeWindow, RegistryPlugin, RenderMetricsPlugin, RequestPlot
};


/// wraps the bevy_metrics_dashboard DashboardWindow
#[derive(Debug, Clone, Default, Component)]
#[require(DashboardWindow(|| DashboardWindow::new("Metrics")))] 
pub struct MetricsWindow;
impl EditorWindow for MetricsWindow {
    fn ui(&self, world: &mut World, cx: EditorWindowContext, ui: &mut egui::Ui) {
        world.run_system_cached_with(draw_all, (cx.entity, ui)).unwrap();
    }
}

// TODO move to core crate
pub struct EditorInputs<'a>{
    pub entity: Entity, 
    pub ui: &'a mut Ui
}
impl SystemInput for EditorInputs<'_>{
    type Param<'i> = EditorInputs<'i>;
    type Inner<'i> = (Entity, &'i mut Ui);

    fn wrap((entity, ui): Self::Inner<'_>) -> Self::Param<'_> {
        EditorInputs{entity, ui}
    }
}

/// Bevy system that draws all [`DashboardWindow`] entities into the
/// [`bevy_egui::EguiContexts`].
///
/// Also handles [`RequestPlot`] events by creating a new plot in each window.
pub fn draw_all(
    // (In(window), InMut(ui)): (In<Entity>,InMut<Ui>),
    input: EditorInputs,
    mut commands: Commands,
    registry: Res<MetricsRegistry>,
    mut cached_configs: ResMut<CachedPlotConfigs>,
    mut requests: EventReader<RequestPlot>,
    mut windows: Query<(Entity, &mut DashboardWindow)>,
) {
    let ui = input.ui;
    let Ok((entity, mut window)) = windows.get_mut(input.entity) else {
        error!("missing window {}", input.entity);
        return;
    };

    let requests: Vec<_> = requests.read().cloned().collect();
    for RequestPlot { key, unit } in requests.iter().cloned() {
        window.add_plot(&registry, &cached_configs, key, unit);
    }

    ui.horizontal(|ui| {
        window.plot_selected_search_result(&registry, &cached_configs, ui);
        if ui.button("Browse").clicked() {
            commands.spawn(NamespaceTreeWindow::new("Namespace Viewer"));
        }
    });
    ui.collapsing("Global Settings", |ui| {
        window.configure_ui(ui);
    });
    ui.separator();
    window.draw_plots(&mut cached_configs, ui);
}

impl Plugin for MetricsWindow {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_editor_window::<Self>();
        if !app.is_plugin_added::<RegistryPlugin>(){
            app.add_plugins(RegistryPlugin::new());
        }
        // TODO: non-generic, move into game repo 
        if !app.is_plugin_added::<CoreMetricsPlugin>(){
            app.add_plugins(CoreMetricsPlugin);
        }
        if !app.is_plugin_added::<RenderMetricsPlugin>(){
            app.add_plugins(RenderMetricsPlugin);
        }


        // https://github.com/bonsairobo/bevy_metrics_dashboard/blob/f4dcff0a2732b2ec6d7c4c924d258c327c9be9c5/src/dashboard_plugin.rs#L15
        app.add_event::<RequestPlot>()
            .init_resource::<CachedPlotConfigs>()
            .add_systems(
                Update,
                (NamespaceTreeWindow::draw_all),
            )
            // Enforce strict ordering:
            // metrics producers (before Last) --> metrics consumers --> bucket clearing
            .add_systems(
                Last,
                DashboardWindow::update_plots_on_all_windows.before(ClearBucketsSystem),
            );
    }
    fn finish(&self, app: &mut App) {
        if app.is_plugin_added::<DashboardPlugin>() {
            error!("plugins {} and {} are incompatible, only add one", type_name::<Self>(), type_name::<DashboardPlugin>());
        }
    }
}
