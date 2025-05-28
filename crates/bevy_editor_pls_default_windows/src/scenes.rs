use bevy::prelude::*;
use bevy_editor_pls_core::{
    editor_window::{EditorWindow, EditorWindowContext},
    AddEditorWindow,
};
use bevy_egui::egui;
//  use bevy_inspector_egui::{bevy_inspector::ui_for_scenes, egui};

#[derive(Clone, Debug, Default, Component, Reflect)]
#[reflect(Component)]
pub struct SceneWindow;

impl EditorWindow for SceneWindow {
    fn ui(&self, world: &mut World, mut cx: EditorWindowContext, ui: &mut egui::Ui) {
        // ui_for_scenes(world, ui);
        warn_once!("unimplemented");
    }
}

impl Plugin for SceneWindow {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_editor_window::<Self>();
    }
}
