use std::alloc::System;
use std::any::{Any, TypeId};

use bevy::ecs::system::SystemChangeTick;
use bevy::utils::hashbrown::{HashMap, HashSet};
use bevy::window::WindowMode;
use bevy::{prelude::*};
use bevy_inspector_egui::bevy_egui::{egui, EguiContext};
use bevy_trait_query::One;
use egui_dock::{NodeIndex, SurfaceIndex, TabBarStyle, TabIndex};
use indexmap::IndexMap;

use crate::editor_window::{EditorWindow, EditorWindowContext, EditorWindowInstance};

#[non_exhaustive]
#[derive(Event)]
pub enum EditorEvent {
    Toggle { now_active: bool },
    FocusSelected,
}

#[derive(Debug)]
enum ActiveEditorInteraction {
    Viewport,
    Editor,
}

#[derive(Resource)]
pub struct Editor {
    on_window: Entity,
    always_active: bool,

    active: bool,

    pointer_used: bool,
    active_editor_interaction: Option<ActiveEditorInteraction>,
    listening_for_text: bool,
    viewport: egui::Rect,
    window_cache: HashMap<Entity, Box<dyn EditorWindow>>,
}
impl Editor {
    pub fn new(on_window: Entity, always_active: bool) -> Self {
        Editor {
            on_window,
            always_active,

            active: always_active,
            pointer_used: false,
            active_editor_interaction: None,
            listening_for_text: false,
            viewport: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(640., 480.)),
            window_cache: default(),
        }
    }

    pub fn window(&self) -> Entity {
        self.on_window
    }
    pub fn always_active(&self) -> bool {
        self.always_active
    }
    pub fn active(&self) -> bool {
        self.active
    }

    /// Panics if `self.always_active` is true
    pub fn set_active(&mut self, active: bool) {
        if !active && self.always_active {
            warn!("cannot call set_active on always-active editor");
        }

        self.active = active;
    }

    pub fn viegport(&self) -> egui::Rect {
        self.viewport
    }
    pub fn is_in_viewport(&self, pos: egui::Pos2) -> bool {
        self.viewport.contains(pos)
    }

    pub fn pointer_used(&self) -> bool {
        self.pointer_used
            || matches!(
                self.active_editor_interaction,
                Some(ActiveEditorInteraction::Editor)
            )
    }

    pub fn listening_for_text(&self) -> bool {
        self.listening_for_text
    }

    pub fn viewport_interaction_active(&self) -> bool {
        !self.pointer_used
            || matches!(
                self.active_editor_interaction,
                Some(ActiveEditorInteraction::Viewport)
            )
    }
}

// pub(crate) type UiFn =
//     Box<dyn Fn(Entity, &mut World, EditorWindowContext, &mut egui::Ui) + Send + Sync + 'static>;
// pub(crate) type EditorWindowState = Box<dyn Any + Send + Sync>;

// struct EditorWindowData {
//     fns: Box<dyn EditorWindow>
// }

#[derive(Resource)]
pub struct EditorTabs {
    pub state: egui_dock::DockState<TreeTab>,
    // NOTE egui dock supports multibple surfaces so why do we need this?
    // pub(crate) floating_windows: Vec<FloatingWindow>,
    // next_floating_window_id: u32,
}

impl Default for EditorTabs {
    fn default() -> Self {
        Self {
            state: egui_dock::DockState::new(vec![]),
            // floating_windows: Default::default(),
            // next_floating_window_id: Default::default(),
        }
    }
}

// TODO perhaps replace with just Entity
#[derive(Clone, Copy, Deref)]
pub struct TreeTab {
    pub entity: Entity,
}
impl From<Entity> for TreeTab {
    fn from(entity: Entity) -> Self {
        Self { entity }
    }
}

impl EditorTabs {
    // pub fn push_to_focused_leaf<W: EditorWindow>(&mut self) {
    //     self.state
    //         .push_to_focused_leaf(TreeTab::CustomWindow(TypeId::of::<W>()));
    //     if let Some((surface_index, node_index)) = self.state.focused_leaf() {
    //         self.state
    //             .set_active_tab((surface_index, node_index, TabIndex(0)));
    //     };
    // }

    // pub fn split<W: EditorWindow>(
    //     &mut self,
    //     parent: NodeIndex,
    //     split: egui_dock::Split,
    //     fraction: f32,
    // ) -> [NodeIndex; 2] {
    //     let node = egui_dock::Node::leaf(TreeTab::CustomWindow(TypeId::of::<W>()));
    //     self.state
    //         .split((SurfaceIndex::main(), parent), split, fraction, node)
    // }

    // pub fn split_right<W: EditorWindow>(
    //     &mut self,
    //     parent: NodeIndex,
    //     fraction: f32,
    // ) -> [NodeIndex; 2] {
    //     self.split::<W>(parent, egui_dock::Split::Right, fraction)
    // }
    // pub fn split_left<W: EditorWindow>(
    //     &mut self,
    //     parent: NodeIndex,
    //     fraction: f32,
    // ) -> [NodeIndex; 2] {
    //     self.split::<W>(parent, egui_dock::Split::Left, fraction)
    // }
    // pub fn split_above<W: EditorWindow>(
    //     &mut self,
    //     parent: NodeIndex,
    //     fraction: f32,
    // ) -> [NodeIndex; 2] {
    //     self.split::<W>(parent, egui_dock::Split::Above, fraction)
    // }
    // pub fn split_below<W: EditorWindow>(
    //     &mut self,
    //     parent: NodeIndex,
    //     fraction: f32,
    // ) -> [NodeIndex; 2] {
    //     self.split::<W>(parent, egui_dock::Split::Below, fraction)
    // }

    // pub fn split_many(
    //     &mut self,
    //     parent: NodeIndex,
    //     fraction: f32,
    //     split: egui_dock::Split,
    //     windows: &[TypeId],
    // ) -> [NodeIndex; 2] {
    //     let tabs = windows.iter().copied().map(TreeTab::CustomWindow).collect();
    //     let node = egui_dock::Node::leaf_with(tabs);
    //     self.state
    //         .split((SurfaceIndex::main(), parent), split, fraction, node)
    // }
}

impl Editor {
    pub fn add_window<W: EditorWindow>(&mut self) {
        // let type_id = std::any::TypeId::of::<W>();
        // let ui_fn = Box::new(ui_fn::<W>);
        // let menu_ui_fn = Box::new(menu_ui_fn::<W>);
        // let viewport_toolbar_ui_fn = Box::new(viewport_toolbar_ui_fn::<W>);
        // let viewport_ui_fn = Box::new(viewport_ui_fn::<W>);
        // let data = EditorWindowData {
        //     ui_fn,
        //     menu_ui_fn,
        //     viewport_toolbar_ui_fn,
        //     viewport_ui_fn,
        //     name: W::NAME,
        //     default_size: W::DEFAULT_SIZE,
        // };
        // if self.windows.insert(type_id, data).is_some() {
        //     panic!(
        //         "window of type {} already inserted",
        //         std::any::type_name::<W>()
        //     );
        // }
        // self.window_states
        //     .insert(type_id, Box::<<W as EditorWindow>::State>::default());
    }
}

impl Editor {
    pub(crate) fn system(world: &mut World) {
        world.resource_scope(|world, mut editor: Mut<Editor>| {
            let Ok(mut egui_context) = world
                .query::<&mut EguiContext>()
                .get_mut(world, editor.on_window)
            else {
                return;
            };
            let egui_context = egui_context.get_mut().clone();

            world.resource_scope(|world, mut editor_internal_state: Mut<EditorTabs>| {
                // TODO move to own system or observer or hook
                let tabs: HashSet<Entity> = editor_internal_state
                    .state
                    .iter_all_tabs()
                    .map(|a| a.1.entity)
                    .collect();

                let mut windows =
                    world.query::<(Entity, &EditorWindowInstance, One<&dyn EditorWindow>)>();
                for (entity, _, methods) in windows.iter(&world) {
                    // design considerations:
                    // no matter what ui() cannot be passed &mut World and &self, if self is in ECS
                    // no matter what it must be passed self here, since dyn doesn't support static methods
                    // we can either maintain a type_id map, or we can dyn clone + bevy_trait_query
                    // the later was more drop-in
                    match editor.window_cache.get_mut(&entity) {
                        Some(v) => {
                            if methods.is_changed() {
                                *v = dyn_clone::clone_box(&*methods);
                            }
                        }
                        None => {
                            editor
                                .window_cache
                                .insert(entity, dyn_clone::clone_box(&*methods));
                        }
                    }

                    if !tabs.contains(&entity) {
                        editor_internal_state
                            .state
                            .main_surface_mut()
                            .push_to_focused_leaf(TreeTab { entity });
                    }
                }

                let last_change_tick = world.last_change_tick();
                let change_tick = world.change_tick();
                editor_internal_state.state.retain_tabs(|t| {
                    windows.contains(t.entity, &world, last_change_tick, change_tick)
                });
                editor.window_cache.retain(|entity, _| {
                    windows.contains(*entity, &world, last_change_tick, change_tick)
                });

                world.resource_scope(|world, mut editor_events: Mut<Events<EditorEvent>>| {
                    editor.editor_ui(
                        world,
                        &egui_context,
                        &mut editor_internal_state,
                        &mut editor_events,
                    );
                });
            });
        });
    }

    fn editor_ui(
        &mut self,
        world: &mut World,
        ctx: &egui::Context,
        internal_state: &mut EditorTabs,
        editor_events: &mut Events<EditorEvent>,
    ) {
        self.editor_menu_bar(world, ctx, internal_state, editor_events);

        if !self.active {
            // self.editor_floating_windows(world, ctx, internal_state);
            self.pointer_used = ctx.wants_pointer_input();
            return;
        }

        let mut tree = std::mem::replace(
            &mut internal_state.state,
            egui_dock::DockState::new(Vec::new()),
        );

        egui_dock::DockArea::new(&mut tree)
            .style(egui_dock::Style {
                tab_bar: TabBarStyle {
                    bg_fill: ctx.style().visuals.window_fill(),
                    ..default()
                },
                ..egui_dock::Style::from_egui(ctx.style().as_ref())
            })
            .show(
                ctx,
                &mut TabViewer {
                    editor: self,
                    internal_state,
                    world,
                },
            );
        internal_state.state = tree;

        let pointer_pos = ctx.input(|input| input.pointer.interact_pos());
        self.pointer_used = pointer_pos.map_or(false, |pos| !self.is_in_viewport(pos));

        // self.editor_floating_windows(world, ctx, internal_state);

        self.listening_for_text = ctx.wants_keyboard_input();

        let is_pressed = ctx.input(|input| input.pointer.press_start_time().is_some());
        match (&self.active_editor_interaction, is_pressed) {
            (_, false) => self.active_editor_interaction = None,
            (None, true) => {
                self.active_editor_interaction = Some(match self.pointer_used {
                    true => ActiveEditorInteraction::Editor,
                    false => ActiveEditorInteraction::Viewport,
                });
            }
            (Some(_), true) => {}
        }
    }

    fn editor_menu_bar(
        &mut self,
        world: &mut World,
        ctx: &egui::Context,
        internal_state: &mut EditorTabs,
        editor_events: &mut Events<EditorEvent>,
    ) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            let bar_response = egui::menu::bar(ui, |ui| {
                if !self.always_active && play_pause_button(self.active, ui).clicked() {
                    self.active = !self.active;
                    editor_events.send(EditorEvent::Toggle {
                        now_active: self.active,
                    });
                }

                //TODO
                // ui.menu_button("Open window", |ui| {
                //     for (&_, window) in self.windows.iter() {
                //         let cx = EditorWindowContext {
                //             entity: Entity::PLACEHOLDER,
                //             internal_state,
                //         };
                //         (window.menu_ui_fn)(world, cx, ui);
                //     }
                // });
            })
            .response;
            // .interact(egui::Sense::click());

            if bar_response.double_clicked() {
                let mut window = world
                    .query::<&mut Window>()
                    .get_mut(world, self.on_window)
                    .unwrap();

                match window.mode {
                    WindowMode::Windowed => {
                        window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Current)
                    }
                    _ => window.mode = WindowMode::Windowed,
                }
            }
        });
    }
}

struct TabViewer<'a> {
    editor: &'a mut Editor,
    internal_state: &'a mut EditorTabs,
    world: &'a mut World,
}
impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = TreeTab;

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let cx = EditorWindowContext {
            // window_states: &mut self.window_states,
            entity: tab.entity,
            internal_state: self.internal_state,
        };

        // design considerations:
        // no matter what ui() cannot be passed &mut World and &self, if self is in ECS
        // no matter what it must be passed self here, since dyn doesn't support static methods
        // we can either maintain a type_id map, or we can dyn clone + bevy_trait_query
        // the later was more drop-in

        self.editor.window_cache[&tab.entity].ui(self.world, cx, ui);
    }

    fn context_menu(
        &mut self,
        ui: &mut egui::Ui,
        tab: &mut Self::Tab,
        _surface: SurfaceIndex,
        _node: NodeIndex,
    ) {
        if ui.button("Pop out").clicked() {
            // TODO
            // if let TreeTab::CustomWindow(window) = tab {
            //     let id = internal_state.next_floating_window_id();
            //     internal_state.floating_windows.push(FloatingWindow {
            //         window,
            //         id,
            //         initial_position: None,
            //     });
            // }

            ui.close_menu();
        }
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        self.editor.window_cache[&tab.entity].name().into()
    }

    fn clear_background(&self, tab: &Self::Tab) -> bool {
        self.editor.window_cache[&tab.entity].clear_background()
    }

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(tab.entity)
    }
}

fn play_pause_button(active: bool, ui: &mut egui::Ui) -> egui::Response {
    let icon = match active {
        true => "▶",
        false => "⏸",
    };
    ui.add(egui::Button::new(icon).frame(false))
}
