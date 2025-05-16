use std::{cell::RefCell, hash::Hash, rc::Rc, sync::Arc};

use bevy::{
    animation::graph,
    prelude::*,
    render::render_graph::{self, RenderGraph},
};
use bevy_editor_pls_core::{
    editor_window::{EditorWindow, EditorWindowContext},
    AddEditorWindow,
};
use bevy_egui::egui::{Checkbox, Id, Ui};
use egui_graphs::{
    to_graph,
    DefaultEdgeShape,
    DefaultNodeShape,
    Graph,
    GraphView,
    LayoutHierarchical,
    LayoutStateHierarchical,
    SettingsInteraction,
    SettingsNavigation,
    SettingsStyle,
};
use parking_lot::{ArcMutexGuard, Mutex};
use petgraph::prelude::StableGraph;

#[derive(Debug, Clone, Reflect, Default, Component)]
#[reflect(Component)]
pub struct SystemGraphWindow();

#[derive(Debug, Clone, Resource)]
pub struct TestGraph(MyGraph);

type MyGraph =
    Graph<&'static str, &'static str, petgraph::Directed, u32, DefaultNodeShape, DefaultEdgeShape>;

impl Plugin for SystemGraphWindow {
    fn build(&self, app: &mut App) {
        app.add_editor_window::<Self>();
        app.add_systems(Startup, |mut commands: Commands| {
            commands.spawn(SystemGraphWindow());
        });

        let graph = generate_graph();
        let graph: MyGraph = to_graph(&graph);

        app.insert_resource(TestGraph(graph));
    }
}

impl EditorWindow for SystemGraphWindow {
    fn ui(&self, world: &mut World, cx: EditorWindowContext, ui: &mut Ui) {
        // let rg = world.resource::<RenderGraph>();

        // let rendergraph_settings = bevy_mod_debugdump::render_graph::settings::Settings::default();
        // let render_graph = bevy_mod_debugdump::render_graph::render_graph_dot(rg, &rendergraph_settings);
        // let render_graph = render_graph.as_str();

        // let g : StableGraph<_,_> = petgraph::dot::dot_parser::ParseFromDot::try_from(render_graph).unwrap();
        // let g = egui_graphs::Graph::from(&g);

        // let g = generate_graph();
        // let mut g : Graph = Graph::from(&g);

        #[rustfmt::skip]
        let (reset, fit, labels) = ui
            .horizontal(|ui| {
                (
                    ui.button("reset").clicked(), 
                    ui.button("fit").clicked(), 
                    {
                        let checked = &mut get_egui_thing(ui, "labels");
                        ui.checkbox(checked, "labels");
                        **checked
                    }
                )
            })
            .inner;

        let interaction_settings = &SettingsInteraction::new()
            .with_dragging_enabled(true)
            .with_node_clicking_enabled(true)
            .with_node_selection_enabled(true)
            .with_node_selection_multi_enabled(true)
            .with_edge_clicking_enabled(true)
            .with_edge_selection_enabled(true)
            .with_edge_selection_multi_enabled(true);

        let nav_settings = &SettingsNavigation::new()
            .with_fit_to_screen_enabled(fit)
            .with_zoom_and_pan_enabled(true)
            .with_screen_padding(0.3)
            .with_zoom_speed(0.1);

        if reset {
            ui.data_mut(|data| {
                data.insert_persisted(
                    Id::new("egui_grpahs_layout"),
                    LayoutStateHierarchical::default(),
                );
                // XXX lib has a bug, this needs to be a auto_id
                // TODO why doesn't this work.
            });
        }

        let style_settings = &SettingsStyle::new().with_labels_always(labels);

        let mut graph = world.resource_mut::<TestGraph>();
        let graph = &mut graph.0;

        // TODO make GraphView take Layouts in builder
        let mut view: GraphView<_, _, _, _, _, _, LayoutStateHierarchical, LayoutHierarchical> =
            GraphView::new(graph)
                .with_interactions(interaction_settings)
                .with_navigations(nav_settings)
                .with_styles(style_settings);

        ui.add(&mut view);
    }
}

// really wish there was a better way than this
fn get_egui_thing<T: 'static + Default + Send + Sync>(
    ui: &Ui,
    salt: impl Hash,
) -> ArcMutexGuard<parking_lot::RawMutex, T> {
    let id = ui.auto_id_with(salt);
    let guard = ui
        .data_mut(|t| {
            t.get_temp_mut_or_default::<Arc<Mutex<T>>>(id)
                .try_lock_arc()
        })
        .expect("already took this data");
    guard
}

fn generate_graph() -> StableGraph<&'static str, &'static str> {
    let mut g = StableGraph::new();

    let a = g.add_node("A");
    let b = g.add_node("B");
    let c = g.add_node("C");
    let d = g.add_node("C");
    let e = g.add_node("C");

    g.add_edge(a, b, "1");
    g.add_edge(a, c, "2");
    g.add_edge(e, a, "3");
    g.add_edge(b, d, "4");

    g
}
