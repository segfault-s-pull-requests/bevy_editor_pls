use bevy::{
    app::Plugin,
    ecs::component::Component,
    prelude::{AppTypeRegistry, ReflectResource, World},
    reflect::TypeRegistry,
};
use bevy_editor_pls_core::{
    editor_window::{EditorWindow, EditorWindowContext},
    AddEditorWindow,
};
use bevy_inspector_egui::egui;

use crate::inspector::{InspectorSelection, InspectorState, InspectorWindow};

#[derive(Debug, Clone, Default, Component)]
pub struct ResourcesWindow;

impl EditorWindow for ResourcesWindow {
    fn ui(&self, world: &mut World, mut cx: EditorWindowContext, ui: &mut egui::Ui) {
        let type_registry = world.resource::<AppTypeRegistry>().clone(); //is Arc
        let type_registry = type_registry.read();
        let mut selection = cx
            .get_mut::<InspectorState>(world)
            .unwrap()
            .map_unchanged(|a| &mut a.selected);

        select_resource(ui, &type_registry, &mut selection);
    }
}

impl Plugin for ResourcesWindow {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_editor_window::<Self>();
    }
}

fn select_resource(
    ui: &mut egui::Ui,
    type_registry: &TypeRegistry,
    selection: &mut InspectorSelection,
) {
    let mut resources: Vec<_> = type_registry
        .iter()
        .filter(|registration| registration.data::<ReflectResource>().is_some())
        .map(|registration| {
            (
                registration.type_info().type_path_table().short_path(),
                registration.type_id(),
            )
        })
        .collect();
    resources.sort_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b));

    for (resource_name, type_id) in resources {
        let selected = match *selection {
            InspectorSelection::Resource(selected, _) => selected == type_id,
            _ => false,
        };

        if ui.selectable_label(selected, resource_name).clicked() {
            *selection = InspectorSelection::Resource(type_id, resource_name.to_owned());
        }
    }
}
