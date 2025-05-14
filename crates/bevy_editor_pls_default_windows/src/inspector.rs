use std::any::TypeId;

use crate::hierarchy::HierarchyState;

// use super::add::{AddWindow, AddWindowState};
use super::hierarchy::HierarchyWindow;
use bevy::app::Plugin;
use bevy::asset::UntypedAssetId;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entities;
use bevy::ecs::reflect;
use bevy::prelude::{AppTypeRegistry, Entity, World};
use bevy::reflect::{Reflect, TypePath, TypeRegistry};
use bevy_editor_pls_core::editor_window::{DefaultLink, EditorWindow, EditorWindowContext, Link};
use bevy_editor_pls_core::AddEditorWindow;
use bevy_inspector_egui::bevy_inspector::hierarchy::SelectedEntities;
use bevy_inspector_egui::{bevy_inspector, egui};

// TODO cant make reflect because of UnTypedAssetId
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub enum InspectorSelection {
    #[default]
    Entities,
    Resource(TypeId, String),
    Asset(TypeId, String, UntypedAssetId),
}

#[derive(Debug, Clone, Default, Component, TypePath)]
pub struct InspectorState {
    pub selected: InspectorSelection,
}

#[derive(Debug, Default, Component, Clone, Copy)]
pub struct InspectorWindow;
impl EditorWindow for InspectorWindow {
    fn ui(&self, world: &mut World, cx: EditorWindowContext, ui: &mut egui::Ui) {
        let type_registry = world.resource::<AppTypeRegistry>().0.clone();
        let type_registry = type_registry.read();

        // now the problem is how to get the data we need.
        // it is compounded by the problem of
        // 1. needing to retain access to &mut world
        // 2. how do ui's interact. Since we no longer have singletons.
        //      could have Default<WindowState> as resouce.
        //      could do manual plumbing.
        // but a key thing here is it isn't that complicated.

        let selected = &cx.get::<InspectorState>(world).unwrap().selected.clone(); // TODO don't clone
        let entities = &cx.get::<HierarchyState>(world).unwrap().selected.clone();

        // let add_window_state = cx.state::<AddWindow>();
        inspector(
            world,
            selected,
            entities,
            ui,
            // add_window_state,
            &type_registry,
        );
    }
}

impl Plugin for InspectorWindow {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_editor_window::<Self>();
        app.register_type::<Link<InspectorState>>();
        app.init_resource::<DefaultLink<InspectorState>>(); 
    }
}

fn inspector(
    world: &mut World,
    selected: &InspectorSelection,
    selected_entities: &SelectedEntities,
    ui: &mut egui::Ui,
    // add_window_state: Option<&AddWindowState>,
    type_registry: &TypeRegistry,
) {
    egui::ScrollArea::vertical().show(ui, |ui| match *selected {
        InspectorSelection::Entities => match selected_entities.as_slice() {
            [] => {
                ui.label("No entity selected");
            }
            &[entity] => {
                bevy_inspector::ui_for_entity(world, entity, ui);
                // add_ui(ui, &[entity], world, add_window_state);
            }
            entities => {
                bevy_inspector::ui_for_entities_shared_components(world, entities, ui);
                // add_ui(ui, entities, world, add_window_state);
            }
        },
        InspectorSelection::Resource(type_id, ref name) => {
            ui.label(name);
            bevy_inspector::by_type_id::ui_for_resource(world, type_id, ui, name, type_registry)
        }
        InspectorSelection::Asset(type_id, ref name, handle) => {
            ui.label(name);
            bevy_inspector::by_type_id::ui_for_asset(world, type_id, handle, ui, type_registry);
        }
    });
}

// fn add_ui(
//     ui: &mut egui::Ui,
//     entities: &[Entity],
//     world: &mut World,
//     add_window_state: Option<&AddWindowState>,
// ) {
//     if let Some(add_window_state) = add_window_state {
//         let layout = egui::Layout::top_down(egui::Align::Center).with_cross_justify(true);
//         ui.with_layout(layout, |ui| {
//             ui.menu_button("+", |ui| {
//                 if let Some(add_item) = crate::add::add_ui(ui, add_window_state) {
//                     for entity in entities {
//                         add_item.add_to_entity(world, *entity);
//                     }
//                 }
//             });
//         });
//     }
// }

pub fn label_button(ui: &mut egui::Ui, text: &str, text_color: egui::Color32) -> bool {
    ui.add(egui::Button::new(egui::RichText::new(text).color(text_color)).frame(false))
        .clicked()
}
