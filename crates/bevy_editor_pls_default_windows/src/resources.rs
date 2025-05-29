use bevy::{
    app::Plugin,
    ecs::component::{Component, ComponentInfo, Components},
    prelude::{AppTypeRegistry, ReflectResource, World},
    reflect::TypeRegistry,
};
use bevy_editor_pls_core::{
    editor_window::{EditorWindow, EditorWindowContext},
    AddEditorWindow,
};
use bevy_inspector_egui::egui;

use crate::inspector::{InspectorSelection, InspectorState};

#[derive(Debug, Clone, Default, Component)]
pub struct ResourcesWindow;

impl EditorWindow for ResourcesWindow {
    fn ui(&self, world: &mut World, cx: EditorWindowContext, ui: &mut egui::Ui) {
        let type_registry = world.resource::<AppTypeRegistry>().clone(); //is Arc
        let type_registry = type_registry.read();

        let name = |r: &ComponentInfo| {
            let type_id = r.type_id()?;
            let info = type_registry.get(type_id)?.type_info();
            let path = info.type_path_table().short_path();
            Some(path)
        };

        // aliased mut issue requires cloning here and disallows splitting this into a reusable function
        // alt is to wrap InspectorState in Arc Mutex
        let mut resources: Vec<(ComponentInfo, String)> = world
            .components()
            .iter_registered()
            .filter(|c|
                // is a resource 
                world.contains_resource_by_id(c.id())
                // is in type registry (otherwise we get lots on noise)
                && c.type_id().is_some_and(|t| type_registry.contains(t)))
            .map(|r| (r.clone(), name(r).unwrap_or_else(|| r.name()).to_string()))
            .collect();
        resources.sort_by(|r1, r2| r1.1.cmp(&r2.1));

        let mut selection = cx
            .get_mut::<InspectorState>(world)
            .unwrap()
            .map_unchanged(|a| &mut a.selected);

        for (r, name) in resources {
            let selected = match *selection {
                InspectorSelection::Resource(selected, _) => selected == r.id(),
                _ => false,
            };

            if ui.selectable_label(selected, &name).clicked() {
                *selection = InspectorSelection::Resource(r.id(), name.to_string());
            }
        }
    }
}

impl Plugin for ResourcesWindow {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_editor_window::<Self>();
    }
}
