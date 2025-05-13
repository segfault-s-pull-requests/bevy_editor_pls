use bevy::{
    app::Plugin,
    asset::ReflectAsset,
    ecs::component::Component,
    prelude::{AppTypeRegistry, World},
    reflect::TypeRegistry,
};
use bevy_editor_pls_core::{
    editor_window::{EditorWindow, EditorWindowContext},
    AddEditorWindow,
};
use bevy_inspector_egui::egui;

use crate::inspector::{InspectorSelection, InspectorState, InspectorWindow};

#[derive(Debug, Default, Clone, Copy, Component)]
pub struct AssetsWindow;

impl EditorWindow for AssetsWindow {
    fn ui(&self, world: &mut World, cx: EditorWindowContext, ui: &mut egui::Ui) {
        let mut selection = cx.get::<InspectorState>(world).unwrap().clone();

        let type_registry = world.resource::<AppTypeRegistry>();
        let type_registry = type_registry.read();

        select_asset(ui, &type_registry, world, &mut selection.selected);
        drop(type_registry);

        let mut r = cx.get_mut::<InspectorState>(world).unwrap();
        r.selected = selection.selected;
    }
}
impl Plugin for AssetsWindow {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_editor_window::<Self>();
    }
}

fn select_asset(
    ui: &mut egui::Ui,
    type_registry: &TypeRegistry,
    world: &World,
    selection: &mut InspectorSelection,
) {
    let mut assets: Vec<_> = type_registry
        .iter()
        .filter_map(|registration| {
            let reflect_asset = registration.data::<ReflectAsset>()?;
            Some((
                registration.type_info().type_path_table().short_path(),
                registration.type_id(),
                reflect_asset,
            ))
        })
        .collect();
    assets.sort_by(|(name_a, ..), (name_b, ..)| name_a.cmp(name_b));

    for (asset_name, asset_type_id, reflect_asset) in assets {
        let handles: Vec<_> = reflect_asset.ids(world).collect();

        ui.collapsing(format!("{asset_name} ({})", handles.len()), |ui| {
            for handle in handles {
                let selected = match *selection {
                    InspectorSelection::Asset(_, _, selected_id) => selected_id == handle,
                    _ => false,
                };

                if ui
                    .selectable_label(selected, format!("{:?}", handle))
                    .clicked()
                {
                    *selection =
                        InspectorSelection::Asset(asset_type_id, asset_name.to_owned(), handle);
                }
            }
        });
    }
}
