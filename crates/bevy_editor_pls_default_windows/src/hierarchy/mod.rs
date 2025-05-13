// pub mod picking;

use bevy::ecs::entity::Entities;
use bevy::pbr::wireframe::Wireframe;
use bevy::prelude::*;
use bevy::reflect::TypeRegistry;
use bevy::render::sync_world::RenderEntity;
use bevy::render::{Extract, RenderApp};
use bevy_editor_pls_core::{editor, AddEditorWindow};
use bevy_inspector_egui::bevy_inspector::guess_entity_name;
use bevy_inspector_egui::bevy_inspector::hierarchy::SelectedEntities;
use bevy_inspector_egui::egui::text::CCursorRange;
use bevy_inspector_egui::egui::{self, ScrollArea};

use bevy_editor_pls_core::{
    editor_window::{EditorWindow, EditorWindowContext},
    Editor,
};
// use bevy_mod_picking::backends::egui::EguiPointer;
// use bevy_mod_picking::prelude::{IsPointerEvent, PointerClick, PointerButton};

// use crate::add::{add_ui, AddWindow, AddWindowState};
use crate::debug_settings::{DebugSettings, DebugSettingsWindow};
use crate::inspector::{InspectorSelection, InspectorState, InspectorWindow};

#[derive(Component)]
pub struct HideInEditor;

#[derive(Debug, Copy, Clone, Component, Default)]
pub struct HierarchyWindow;
impl EditorWindow for HierarchyWindow {
    fn ui(&self, world: &mut World, mut cx: EditorWindowContext, ui: &mut egui::Ui) {
        let mut hierarchy_state = cx.get::<HierarchyState>(world).unwrap().clone();

        ScrollArea::vertical().show(ui, |ui| {
            let type_registry = world.resource::<AppTypeRegistry>().clone();
            let type_registry = type_registry.read();
            let new_selected = Hierarchy {
                world,
                state: &mut hierarchy_state,
                type_registry: &type_registry,
                // add_state: add_state.as_deref(),
            }
            .show(ui);

            if new_selected {
                let mut v = cx.get_mut::<InspectorState>(world).unwrap();
                v.selected = InspectorSelection::Entities;
            }
            let mut v = cx.get_mut::<HierarchyState>(world).unwrap();
            *v.as_mut() = hierarchy_state;
        });
    }
}

impl Plugin for HierarchyWindow {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_editor_window::<HierarchyWindow>();

        // picking::setup(app);
        app.add_systems(PostUpdate, clear_removed_entites);
        // .add_system(handle_events);

        app.sub_app_mut(RenderApp)
            .add_systems(ExtractSchedule, extract_wireframe_for_selected);
    }
}

fn clear_removed_entites(mut state: Query<&mut HierarchyState>, entities: &Entities) {
    for mut state in state.iter_mut() {
        state.selected.retain(|entity| entities.contains(entity));
    }
}

/*fn handle_events(
    mut click_events: EventReader<PointerClick>,
    mut editor: ResMut<Editor>,
    editor_state: Res<EditorState>,
    input: Res<Input<KeyCode>>,
    egui_entity: Query<&EguiPointer>,
    mut egui_ctx: ResMut<EguiContext>,
) {
    for click in click_events.iter() {
        if !editor_state.active {
            return;
        }

        if click.event_data().button != PointerButton::Primary {
            continue;
        }

        if egui_entity.get(click.target()).is_ok() || egui_ctx.ctx_mut().wants_pointer_input() {
            continue;
        };

        let state = editor.window_state_mut::<HierarchyWindow>().unwrap();

        let ctrl = input.any_pressed([KeyCode::LControl, KeyCode::RControl]);
        let shift = input.any_pressed([KeyCode::LShift, KeyCode::RShift]);
        let mode = SelectionMode::from_ctrl_shift(ctrl, shift);

        let entity = click.target();
        info!("Selecting mesh, found {:?}", entity);
        state
            .selected
            .select(mode, entity, |_, _| std::iter::once(entity));
    }
}*/

fn extract_wireframe_for_selected(
    debug: Extract<Res<DebugSettings>>,
    state: Extract<Query<&HierarchyState>>,
    mut commands: Commands,
    query: Extract<Query<RenderEntity>>,
) {
    if debug.highlight_selected {
        for state in state.iter() {
            let selected = &state.selected;
            for selected in selected.iter() {
                if let Ok(r_id) = query.get(selected) {
                    if let Some(mut entity) = commands.get_entity(r_id) {
                        entity.insert(Wireframe);
                    }
                }
            }
        }
    }
}

#[derive(Default, Clone, Component)]
pub struct HierarchyState {
    pub selected: SelectedEntities,
    rename_info: Option<RenameInfo>,
}

#[derive(Debug, Clone)]
pub struct RenameInfo {
    entity: Entity,
    renaming: bool,
    current_rename: String,
}

struct Hierarchy<'a> {
    world: &'a mut World,
    state: &'a mut HierarchyState,
    type_registry: &'a TypeRegistry,
    // add_state: Option<&'a AddWindowState>,
}

impl Hierarchy<'_> {
    fn show(&mut self, ui: &mut egui::Ui) -> bool {
        let mut despawn_recursive = None;
        let mut despawn = None;

        let HierarchyState {
            selected,
            rename_info,
        } = self.state;

        let new_selection = bevy_inspector_egui::bevy_inspector::hierarchy::Hierarchy {
            extra_state: rename_info,
            world: self.world,
            type_registry: self.type_registry,
            selected,
            context_menu: Some(&mut |ui, entity, world, rename_info| {
                if ui.button("Despawn").clicked() {
                    despawn_recursive = Some(entity);
                }

                if ui.button("Remove keeping children").clicked() {
                    despawn = Some(entity);
                }

                if ui.button("Rename").clicked() {
                    let entity_name = guess_entity_name(world, entity);
                    *rename_info = Some(RenameInfo {
                        entity,
                        renaming: true,
                        current_rename: entity_name,
                    });
                    ui.close_menu();
                }

                // if let Some(add_state) = self.add_state {
                //     ui.menu_button("Add", |ui| {
                //         if let Some(add_item) = add_ui(ui, add_state) {
                //             add_item.add_to_entity(world, entity);
                //             ui.close_menu();
                //         }
                //     });
                // }
            }),
            shortcircuit_entity: Some(&mut |ui, entity, world, rename_info| {
                if let Some(rename_info) = rename_info {
                    if rename_info.renaming && rename_info.entity == entity {
                        rename_entity_ui(ui, rename_info, world);

                        return true;
                    }
                }

                false
            }),
        }
        .show::<Without<HideInEditor>>(ui);

        if let Some(entity) = despawn_recursive {
            bevy::hierarchy::despawn_with_children_recursive(self.world, entity, true);
        }
        if let Some(entity) = despawn {
            self.world.entity_mut(entity).despawn();
            self.state.selected.remove(entity);
        }

        if ui.input(|input| input.key_pressed(egui::Key::Delete)) {
            for entity in self.state.selected.iter() {
                self.world.entity_mut(entity).despawn_recursive();
            }
            self.state.selected.clear();
        }

        new_selection
    }
}

fn rename_entity_ui(ui: &mut egui::Ui, rename_info: &mut RenameInfo, world: &mut World) {
    use egui::epaint::text::cursor::CCursor;
    use egui::widgets::text_edit::{TextEdit, TextEditOutput};

    let id = egui::Id::new(rename_info.entity);

    let edit = TextEdit::singleline(&mut rename_info.current_rename).id(id);
    let TextEditOutput {
        response,
        state: mut edit_state,
        ..
    } = edit.show(ui);

    // Runs once to end renaming
    if response.lost_focus() {
        rename_info.renaming = false;

        match world.get_entity_mut(rename_info.entity) {
            Ok(mut ent_mut) => match ent_mut.get_mut::<Name>() {
                Some(mut name) => {
                    name.set(rename_info.current_rename.clone());
                }
                None => {
                    ent_mut.insert(Name::new(rename_info.current_rename.clone()));
                }
            },
            Err(err) => {
                error!(?err, "Failed to get renamed entity");
            }
        }
    }

    // Runs once when renaming begins
    if !response.has_focus() {
        response.request_focus();
        edit_state.cursor.set_char_range(Some(CCursorRange::two(
            CCursor::new(0),
            CCursor::new(rename_info.current_rename.len()),
        )));
    }

    TextEdit::store_state(ui.ctx(), id, edit_state);
}
