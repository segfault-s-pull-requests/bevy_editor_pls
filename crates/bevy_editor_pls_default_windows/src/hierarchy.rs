use avian3d::parry::either;
use bevy::color::palettes::css::GREEN;
use bevy::ecs::entity::Entities;
use bevy::ecs::system::SystemIdMarker;
use bevy::pbr::wireframe::Wireframe;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::reflect::TypeRegistry;
use bevy::render::sync_world::RenderEntity;
use bevy::render::{Extract, RenderApp};
use bevy_editor_pls_core::editor::EditorTabs;
use bevy_editor_pls_core::editor_window::{DefaultLink, Link, LinksMut};
use bevy_editor_pls_core::{AddEditorWindow, Editor, EditorSet};
use bevy_egui::EguiContext;
use bevy_inspector_egui::bevy_inspector::guess_entity_name;
use bevy_inspector_egui::bevy_inspector::hierarchy::{SelectedEntities, SelectionMode};
use bevy_inspector_egui::egui::text::CCursorRange;
use bevy_inspector_egui::egui::{self, ScrollArea};

use bevy_editor_pls_core::editor_window::{EditorWindow, EditorWindowContext};
use bevy_mod_outline::{ComputedOutline, OutlineMode, OutlinePlugin, OutlineStencil, OutlineVolume};
// use bevy_mod_picking::backends::egui::EguiPointer;
// use bevy_mod_picking::prelude::{IsPointerEvent, PointerClick, PointerButton};

use crate::cameras::CameraWindow;
// use crate::add::{add_ui, AddWindow, AddWindowState};
use crate::debug_settings::DebugSettings;
use crate::inspector::{InspectorSelection, InspectorState};

#[derive(Component)]
pub struct HideInEditor;

#[derive(Debug, Copy, Clone, Component, Default)]
pub struct HierarchyWindow;
impl EditorWindow for HierarchyWindow {
    fn ui(&self, world: &mut World, cx: EditorWindowContext, ui: &mut egui::Ui) {
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
        if !app.is_plugin_added::<OutlinePlugin>() {
            app.add_plugins(OutlinePlugin);
        }

        app.add_editor_window::<HierarchyWindow>();
        app.register_type::<Link<HierarchyState>>();
        app.init_resource::<DefaultLink<HierarchyState>>();

        app.add_systems(PostUpdate, clear_removed_entites);
        app.add_systems(PostUpdate, handle_events.after(EditorSet::UI));
        app.add_systems(Update, update_outline.after(handle_events));

        app.sub_app_mut(RenderApp)
            .add_systems(ExtractSchedule, extract_wireframe_for_selected);
    }
}

fn clear_removed_entites(mut state: Query<&mut HierarchyState>, entities: &Entities) {
    for mut state in state.iter_mut() {
        state.selected.retain(|entity| entities.contains(entity));
    }
}

/// TODO move this out of heirarchy, because it should work without a hierarchy window open
/// update outlines around entities selected in hierarchy windows
fn update_outline(
    editor: Res<Editor>,
    mut state: LinksMut<HierarchyState>,
    windows: Query<Entity, With<HierarchyWindow>>,
    // mut outlines: Query<&mut OutlineVolume>,
    mut commands: Commands,
    mesh: Query<&Mesh3d>,
    aabb: Query<&Mesh3d>,
) {
    for window in windows.iter() {
        let Some(mut state) = state.get_mut(window) else {
            continue;
        };
        let state = state.as_mut();
        if editor.active {
            for s in state.selected.iter() {
                if !state.outline.contains(&s) {
                    state.outline.insert(s);
                    if mesh.contains(s){
                        commands.entity(s).insert((
                            OutlineVolume {
                                visible: true,
                                width: 4.0,
                                colour: Color::Srgba(Srgba {
                                    red: 1.0,
                                    green: 0.0,
                                    blue: 1.0,
                                    alpha: 0.8,
                                }),
                            },
                            OutlineStencil::default(),
                            OutlineMode::FloodFlatDoubleSided,
                        ));
                    }
                    if aabb.contains(s){
                        commands.entity(s).insert(bevy::gizmos::prelude::ShowAabbGizmo { color: Some(GREEN.into())}); //todo move systems
                    }
                }

            }
        }

        let mut to_remove = Vec::new();
        for s in state.outline.iter() {
            if !state.selected.contains(*s) || !editor.active {
                to_remove.push(*s);
                // BUG: bevy_mod_picking: seems that visible=false outlines still clip other outlines (floor clipping tower)
                commands.entity(*s).remove::<(OutlineVolume, OutlineStencil, OutlineMode, ComputedOutline, ShowAabbGizmo)>();
                // if let Ok(mut a) = outlines.get_mut(*s) {
                //     a.visible = false;
                // }
            }
        }
        for e in to_remove {
            state.outline.remove(&e);
        }
    }
}

fn handle_events(
    mut click_events: EventReader<Pointer<Click>>,
    // mut editor: ResMut<Editor>,
    // editor_state: Res<EditorState>,
    input: Res<ButtonInput<KeyCode>>,
    
    mut egui_ctx: Query<&mut EguiContext>,
    mut state: LinksMut<HierarchyState>,

    editor: Res<Editor>,
    mut tabs: ResMut<EditorTabs>,
    camera_tabs: Query<(Entity, &CameraWindow)>,
) {
    let Some(active_window) = tabs.state.find_active_focused().map(|a| a.1.entity) else {
        return;
    };
    if !(editor.active && camera_tabs.contains(active_window)) {
        click_events.clear();
    }

    for click in click_events.read() {
        if click.event.button != PointerButton::Primary {
            continue;
        }

        if click.target == editor.window() {
            continue;
        }

        let mut ctx = egui_ctx.get_mut(editor.window()).unwrap();
        // bevy_egui should stop picking from passing through egui ui nodes to the viewport.
        // but this doesn't seem to be happening
        // neither of these work, so I'm at a loss.
        let ctx = ctx.get_mut();
        // dbg!(ctx.wants_pointer_input()); //true
        // dbg!(ctx.is_pointer_over_area()); //true
        // dbg!(ctx.is_using_pointer()); //false
        if ctx.wants_pointer_input() || ctx.is_pointer_over_area() {
            // continue;
        };

        let ctrl = input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
        let shift = input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let mode = SelectionMode::from_ctrl_shift(ctrl, shift);

        let entity = click.target;
        info!("Selecting mesh, found {:?}", entity);

        let mut state = state.get_mut(active_window).unwrap();
        state
            .selected
            .select(mode, entity, |_, _| std::iter::once(entity));
    }
}

fn extract_wireframe_for_selected(
    debug: Extract<Res<DebugSettings>>,
    state: Extract<Query<&HierarchyState>>,
    mut commands: Commands,
    query: Extract<Query<RenderEntity>>,
) {
    if debug.physics_gizmos {
        for state in state.iter() {
            let selected = &state.selected;
            for selected in selected.iter() {
                if let Ok(r_id) = query.get(selected) {
                    if let Ok(mut entity) = commands.get_entity(r_id) {
                        entity.insert(Wireframe);
                    }
                }
            }
        }
    }
}

#[derive(Default, Clone, Component, TypePath)]
pub struct HierarchyState {
    pub selected: SelectedEntities,
    rename_info: Option<RenameInfo>,
    outline: HashSet<Entity>,
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
            outline,
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
        .show::<(
            Without<HideInEditor>,
            Without<Observer>,
            Without<SystemIdMarker>,
        )>(ui);

        if let Some(entity) = despawn_recursive {
            self.world.entity_mut(entity).despawn();
        }
        if let Some(entity) = despawn {
            let mut e = self.world.entity_mut(entity);
            e.remove::<Children>();
            e.despawn();
            self.state.selected.remove(entity);
        }

        if ui.input(|input| input.key_pressed(egui::Key::Delete)) {
            for entity in self.state.selected.iter() {
                self.world.entity_mut(entity).despawn();
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
