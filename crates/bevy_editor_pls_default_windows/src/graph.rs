use std::{borrow::Cow, hash::Hash, sync::Arc};

use bevy::{
    ecs::{archetype::ArchetypeComponentId, query::ComponentAccessKind},
    platform::collections::HashMap,
    prelude::*,
    render::render_graph::{self, RenderGraph},
};
use bevy_editor_pls_core::{
    editor_window::{EditorWindow, EditorWindowContext},
    AddEditorWindow,
};
use bevy_egui::egui::{Id, Ui};
type Node = dot_parser::canonical::Node<(String, String)>;
type AList<A = (String, String)> = dot_parser::ast::AList<A>;

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
use petgraph::{csr::DefaultIx, prelude::StableGraph, Directed};

use crate::debug_settings::debugdump::DotGraphs;
#[derive(Debug, Clone, Reflect, Default, Component)]
#[reflect(Component)]
pub struct SystemGraphWindow();

#[derive(Debug, Clone, Default, Resource)]
pub struct MyGraphs {
    // pub render: Option<RenderGraph>,

    //pub systems: TypeMap<>
}

impl Plugin for SystemGraphWindow {
    fn build(&self, app: &mut App) {
        app.add_editor_window::<Self>();
        // app.add_systems(Startup, |mut commands: Commands| {
        //     commands.spawn(SystemGraphWindow());
        // });

        let graph = generate_graph();
        let graph: MyGraph = to_graph(&graph);

        app.insert_resource(TestGraph(graph));
        app.insert_resource(MyGraphs::default());
    }
}

impl EditorWindow for SystemGraphWindow {
    fn ui(&self, world: &mut World, _cx: EditorWindowContext, ui: &mut Ui) {
        // let rg = world.resource::<MyGraphs>();

        // if rg.render.is_none() {
        //     // let rendergraph_settings = bevy_mod_debugdump::render_graph::settings::Settings::default();
        //     // let render_graph = bevy_mod_debugdump::render_graph::render_graph_dot(rg, &rendergraph_settings);
        //     // let render_graph = render_graph.as_str();

        //     fn to_owned(ls: &AList<(&str, &str)>) -> AList{
        //         let elems = ls.elems.iter().map(|(a,b)|(a.to_string(),b.to_string())).collect::<Vec<_>>();
        //         AList {
        //             elems
        //         }
        //     }

        //     let Some(graphs) = world.get_resource::<DotGraphs>() else {
        //         error_once!("missing dot graphs resource");
        //         return
        //     };
        //     let Some(render_graph) = graphs.render_graph.as_ref() else {
        //         error_once!("missing dot graphs render graph");
        //         return;
        //     };

        //     let g : StableGraph<_,_> = match petgraph::dot::dot_parser::ParseFromDot::try_from(render_graph) {
        //         Ok(g) => g,
        //         Err(e) => {
        //             error_once!("parse {}", e);
        //             return;
        //         }
        //     };
        //     let g : StableGraph<Node, AList> = g.map(|_, n| {
        //         Node {
        //             id: n.id.clone(),
        //             port: n.port.clone(),
        //             attr: to_owned(&n.attr)
        //         }
        //     }, |_, e| to_owned(e));

        //     let mut graph : Graph<
        //         Node,
        //         AList,
        //         Directed,
        //         DefaultIx,
        //     > = egui_graphs::Graph::from(&g);

        //     let mut mygraphs = world.resource_mut::<MyGraphs>();
        //     mygraphs.render = Some(graph);
        // }

        // // let g = generate_graph();
        // // let mut g : Graph = Graph::from(&g);

        // #[rustfmt::skip]
        // let (reset, fit, labels) = ui
        //     .horizontal(|ui| {
        //         (
        //             ui.button("reset").clicked(),
        //             ui.button("fit").clicked(),
        //             {
        //                 let checked = &mut get_egui_thing(ui, "labels");
        //                 ui.checkbox(checked, "labels");
        //                 **checked
        //             }
        //         )
        //     })
        //     .inner;

        // let interaction_settings = &SettingsInteraction::new()
        //     .with_dragging_enabled(true)
        //     .with_node_clicking_enabled(true)
        //     .with_node_selection_enabled(true)
        //     .with_node_selection_multi_enabled(true)
        //     .with_edge_clicking_enabled(true)
        //     .with_edge_selection_enabled(true)
        //     .with_edge_selection_multi_enabled(true);

        // let nav_settings = &SettingsNavigation::new()
        //     .with_fit_to_screen_enabled(fit)
        //     .with_zoom_and_pan_enabled(true)
        //     .with_screen_padding(0.3)
        //     .with_zoom_speed(0.1);

        // if reset {
        //     ui.data_mut(|data| {
        //         data.insert_persisted(
        //             Id::new("egui_grpahs_layout"),
        //             LayoutStateHierarchical::default(),
        //         );
        //         // XXX lib has a bug, this needs to be a auto_id
        //         // TODO why doesn't this work.
        //     });
        // }

        // let graph = &mut world.resource_mut::<MyGraphs>().render;
        // let graph = graph.as_mut().unwrap();

        // let style_settings = &SettingsStyle::new().with_labels_always(labels);

        // // TODO make GraphView take Layouts in builder
        // let mut view: GraphView<_, _, _, _, _, _, LayoutStateHierarchical, LayoutHierarchical> =
        //     GraphView::new(graph)
        //         .with_interactions(interaction_settings)
        //         .with_navigations(nav_settings)
        //         .with_styles(style_settings);

        // ui.add(&mut view);
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

#[derive(Debug, Clone, Resource)]
pub struct TestGraph(MyGraph);

type MyGraph =
    Graph<&'static str, &'static str, petgraph::Directed, u32, DefaultNodeShape, DefaultEdgeShape>;

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

// does not implement clone
// fn copy_render_graph(graph: Res<RenderGraph>) {
//     if graph.is_changed() {
//         info!("copy render graph to main world.");
//         graph.clone();
//     }
// }

struct SystemMeta {
    access: Vec<ComponentAccessKind<ArchetypeComponentId>>,
    name: Cow<'static, str>,
}

pub fn setup(app: &App) {
    let schedule = app.get_schedule(Update).unwrap();

    for a in schedule.systems().unwrap() {
        let access =
            if let Ok(access) = a.1.archetype_component_access().try_iter_component_access() {
                let access: Vec<ComponentAccessKind<ArchetypeComponentId>> = access.collect();
                access
            } else {
                vec![]
            };

        let name = a.1.name();

        schedule.graph().hierarchy().graph().edges(a.0);
    }
}
