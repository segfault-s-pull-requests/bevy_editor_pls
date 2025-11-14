use std::f64::NAN;

use avian3d::parry::utils::hashmap::HashMap;
use bevy::{diagnostic::{Diagnostic, DiagnosticPath, DiagnosticsStore}, prelude::*};
use bevy_editor_pls_core::{
    editor_window::{EditorWindow, EditorWindowContext},
    AddEditorWindow,
};
use bevy_inspector_egui::egui;

#[derive(Debug, Clone, Default, Component)]
#[require(DiagWindowState)]
pub struct DiagnosticsWindow;
impl EditorWindow for DiagnosticsWindow {
    fn ui(&self, world: &mut World, cx: EditorWindowContext, ui: &mut egui::Ui) {
        let diagnostics = match world.get_resource::<DiagnosticsStore>() {
            Some(diagnostics) => diagnostics,
            None => {
                ui.label("Diagnostics resource not available");
                return;
            }
        };

        // TODO this pattern sucks
        let mut state = cx.get::<DiagWindowState>(world).unwrap().clone();
        diagnostic_ui(ui, diagnostics, &mut state);
        *cx.get_mut::<DiagWindowState>(world).unwrap() = state;
    }
}
impl Plugin for DiagnosticsWindow {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_editor_window::<Self>();
    }
}

#[derive(Debug, Default)]
struct Data {
    diag: Option<Diagnostic>,
    children: HashMap<String, Data>,
}

#[derive(Debug, Clone, Default, Component)]
pub struct DiagWindowState{
    filter: String,   
}

// NOTE: annoyance with systems. Need better mixing of inputs and systemparams or something. 
// example. here it would be great to use a Local<String> to store the ui.text_edit_singleline
// on the other hand, we need a seperate one per line,
// I guess actually what we want is to allow Windows to be regular components.

fn diagnostic_ui(ui: &mut egui::Ui, diagnostics: &DiagnosticsStore, state: &mut DiagWindowState) {
    egui::Grid::new("frame time diagnostics").show(ui, |ui| {
        ui.text_edit_singleline(&mut state.filter);
        ui.end_row();

        if diagnostics.iter().next().is_none() {
            ui.label(
                r#"No diagnostics found. Possible plugins to add:
            - `FrameTimeDiagnosticsPlugin`
            - `EntityCountDiagnisticsPlugin`
            - `AssetCountDiagnosticsPlugin`
            "#,
            );
            return;
        }

        let mut keys : Vec<Vec<&str>> = diagnostics.iter().map(|d|d.path().components().collect()).collect();
        keys.sort();

        // build table
        for path in keys.iter() {
            let diagnostic = diagnostics.get(&DiagnosticPath::from_components(path.iter().copied())).unwrap();

            if !state.filter.is_empty() && !diagnostic.path().as_str().contains(&state.filter) {
                continue;
            }

            ui.label(diagnostic.path().as_str());
            ui.label(format!("{:.2}", diagnostic.average().unwrap_or(NAN)));
            // ui.add_space(1.0);
            ui.label(format!("{:.2}", diagnostic.smoothed().unwrap_or(NAN)));
            // ui.add_space(1.0);
            ui.label(format!("{:.2}", diagnostic.value().unwrap_or(NAN)));
            // ui.add_space(1.0);
            ui.label(format!("{:.3}", diagnostic.measurement().map(|v|v.time.elapsed().as_secs_f32()).unwrap_or(f32::NAN)));
            ui.end_row();
        }
    });
}
