// pieces needed for editor
// 1. dynamic querys
// 2. persistence
// 3. composability
// 4. command pallete

use std::{
    any::TypeId,
    cell::RefCell,
    cmp::{max, min},
    f64::consts::E,
    ops::{DerefMut, Range, Rem},
    process::Child,
    sync::Arc,
    u8,
};

use avian3d::parry::na;
use bevy::{
    ecs::{
        component::{ComponentId, ComponentInfo},
        observer::TriggerTargets,
        relationship::Relationship,
        system::{FunctionSystem, SystemId},
    },
    platform::collections::HashMap,
    prelude::*,
    reflect::{TypeRegistry, TypeRegistryArc},
    ui,
};
use bevy_egui::egui::{
    self,
    emath::Numeric,
    text::LayoutJob,
    FontId,
    Frame,
    Key,
    KeyboardShortcut,
    LayerId,
    Layout,
    Margin,
    Modifiers,
    Response,
    RichText,
    ScrollArea,
    Sense,
    Stroke,
    TextEdit,
    Ui,
    UiBuilder,
    Vec2,
    WidgetText,
};
use bevy_inspector_egui::{bevy_inspector::guess_entity_name, egui_utils::layout_job};
use parking_lot::Mutex;
use smallvec::SmallVec;
use transform_gizmo_bevy::{Color32, Pos2};

pub mod window {
    use bevy_editor_pls_core::{
        editor_window::{EditorWindow, EditorWindowContext},
        AddEditorWindow,
    };
    use bevy::prelude::*;
    use bevy_egui::egui;
    use bevy_inspector_egui::bevy_inspector::hierarchy::Hierarchy;

    use crate::{hierarchy::HierarchyState, inspector::InspectorState, query::draw_explorer};

    #[derive(Debug, Default, Component, Clone, Copy)]
    pub struct NavWindow;
    impl EditorWindow for NavWindow {
        fn ui(&self, world: &mut World, cx: EditorWindowContext, ui: &mut egui::Ui) {
            // let type_registry = world.resource::<AppTypeRegistry>().0.clone();
            // let type_registry = type_registry.read();

            // now the problem is how to get the data we need.
            // it is compounded by the problem of
            // 1. needing to retain access to &mut world
            // 2. how do ui's interact. Since we no longer have singletons.
            //      could have Default<WindowState> as resouce.
            //      could do manual plumbing.
            // but a key thing here is it isn't that complicated.

            let mut entities = cx.get::<HierarchyState>(world).unwrap().selected.clone();
            let selected_c = cx
                .get::<InspectorState>(world)
                .unwrap()
                .component_selected
                .clone();

            let focused = cx.focused;


            // default selection.
            // these don't persist. meaning that if the scene loads after the editor is open it overrules the second fallback.
            // once user actaully navigates / selects a entity then these won't run.
            // TODO. make the above behaviour less happenstance. 
            if entities.is_empty() {
                if let Some(a) = world
                    .query_filtered::<Entity, With<SceneRoot>>()
                    .iter(&world)
                    .next()
                {
                    entities.entities.push(a);
                } else if let Some(a) = world
                    .query_filtered::<Entity, Without<ChildOf>>()
                    .iter(&world)
                    .next()
                {
                    entities.entities.push(a);
                }
            }

            if let Some(e) = entities.iter().next() {
                if let Some(e) = draw_explorer(world, e, ui, &selected_c, focused) {
                    cx.get_mut::<HierarchyState>(world)
                        .unwrap()
                        .selected
                        .entities = vec![e];
                }
            };

            // let add_window_state = cx.state::<AddWindow>();
        }
    }

    impl Plugin for NavWindow {
        fn build(&self, app: &mut bevy::prelude::App) {
            app.add_editor_window::<Self>();
        }
    }
}

fn search(ui: &mut Ui) {}

/// returns vec including the entity and well as ancestors and descendents such that the returned vec forms a chain of single children.
fn get_bundle(
    entity: In<Entity>,
    parent: Query<&ChildOf>,
    children: Query<&Children>,
) -> SmallVec<[Entity; 4]> {
    let mut ret = SmallVec::new();
    let entity = entity.0;
    ret.push(entity);
    for e in parent.iter_ancestors(entity) {
        if children.get(e).unwrap().len() == 1 {
            ret.insert(0, e);
        } else {
            break;
        }
    }

    let mut current = entity;
    while let Ok(ls) = children.get(current) {
        if ls.len() == 1 {
            current = ls[0];
            ret.push(current);
        } else {
            break;
        }
    }
    ret
}

// TODO take inspiration from https://github.com/idanarye/bevy-egui-kbgp for keybinds

fn draw_explorer(
    world: &mut World,
    entity: Entity,
    ui: &mut Ui,
    selected_c: &Vec<ComponentId>,
    focused: bool,
) -> Option<Entity> {
    let registry = world.resource::<AppTypeRegistry>().clone();

    // disable text selection by default
    ui.style_mut().interaction.selectable_labels = false;

    // let mut get_bundle = IntoSystem::into_system(get_bundle);
    // get_bundle.initialize(world);

    // get bundle for this entity
    let bundle = world.run_system_cached_with(get_bundle, entity).unwrap();
    let bundle_index = bundle
        .iter()
        .enumerate()
        .find(|a| *a.1 == entity)
        .unwrap()
        .0; //TODO cleaner

    // get first parent and then children
    let parent = world.entity(bundle[0]).get::<ChildOf>().map(|a| a.0);
    let mut index = 0;
    let siblings = match parent {
        Some(parent) => {
            let children = world.entity(parent).get::<Children>().unwrap();
            children // gets rid of world borrow
                .into_iter()
                .cloned()
                .collect::<SmallVec<[Entity; 16]>>()
        }
        // None => vec![bundle.clone()], actually we want to treat all root nodes as siblings
        None => {
            // XXX this is really bad if user has alot of root entities
            // in actuality we need to do this lazy everywhere
            // TODO should draw archetypes at top level, not entities. would solve need for Without<Observer>
            // argueably group by archetype everywhere.
            let mut siblings = world
                .query_filtered::<Entity, (
                    Without<ChildOf>,
                    Without<bevy::ecs::system::SystemIdMarker>,
                    Without<Observer>,
                )>()
                .iter(&world)
                .collect::<SmallVec<[Entity; 16]>>();
            siblings.sort(); // order nor guarenteed. Only an issue for root, bc Children has order
            siblings
        }
    };



    // TODO grab this when collecting sibling.
    index = siblings
        .iter()
        .enumerate()
        .find(|a| *a.1 == bundle[0])
        .map(|x| x.0)
        .unwrap_or_default(); // NOTE: don't panic if somehow a filtered entity is selected
    let siblings: Vec<SmallVec<[Entity; 4]>> = siblings
        .into_iter()
        .map(|e| world.run_system_cached_with(get_bundle, e).unwrap())
        .collect();

    // get ancestors, bundlified
    let mut parent_chain: Vec<SmallVec<[Entity; 4]>> = Vec::new();
    let mut current = parent;
    while current.is_some() {
        assert!(parent_chain.iter().all(|c| !c.contains(&current.unwrap())));
        let bundle = world
            .run_system_cached_with(get_bundle, current.unwrap())
            .unwrap();
        current = world.get::<ChildOf>(bundle[0]).map(|c| c.0);
        parent_chain.push(bundle);
    }
    parent_chain.reverse();

    // get children, bundlified
    let mut children_bundled: Vec<EntityBundle> = default();
    let bottom = *bundle.last().unwrap();
    let children = world.get::<Children>(bottom);
    if let Some(children) = children {
        let children = children.into_iter().cloned().collect::<Vec<_>>(); // borrow checker bullshit
        for child in children.into_iter() {
            let bundle = world.run_system_cached_with(get_bundle, child).unwrap();
            children_bundled.push(bundle);
        }
    }

    let f_nest = true;

    // NOTE: This put siblings below ancestors. But this used up too much vertical space, when we have more horizontal.
    // It had a nice arrow in the ancestor stack. But this used up unessecary horizontal space.
    // ui.horizontal(|ui| {
    //     ui.vertical(|ui| {
    //         let mut offset = 0.0;
    //         if parent.is_some() {
    //             let mut n: u8 = 0;
    //             for b in parent_chain {
    //                 ui.horizontal(|ui| {
    //                     if n != 0 && f_nest {
    //                         ui.add_space(offset);
    //                         offset = 15.0 * n as f32;
    //                         ui.label(
    //                             RichText::new("↳").family(bevy_egui::egui::FontFamily::Monospace),
    //                         );
    //                     }
    //                     n += 1;
    //                     draw_bundle(&world, b, entity, ui);
    //                 });
    //             }
    //         }

    //         ui.horizontal(|ui| {
    //             ui.add_space(offset);
    //             if parent.is_some() {
    //                 // ui.label(RichText::new("  ↳").family(bevy_egui::egui::FontFamily::Monospace));
    //             }

    //             let frame = Frame::new()
    //                 .corner_radius(1.0)
    //                 // .stroke(Stroke::new(1.0, Color32::BLACK))
    //                 .outer_margin(Margin {
    //                     left: 15,
    //                     ..default()
    //                 });
    //             // .inner_margin(2.0);
    //             frame.show(ui, |ui| {
    //                 ui.vertical(|ui| {
    //                     let header = draw_header(ui, "siblings");
    //                     for b in siblings {
    //                         ui.horizontal(|ui| {
    //                             // ui.label("-");
    //                             draw_bundle(&world, b, entity, ui);
    //                         });
    //                     }

    //                     draw_line(ui, &header);
    //                 });
    //             });
    //         });
    //     });
    //     ui.vertical(|ui| {
    //         for b in children_bundled {
    //             draw_bundle(&world, b, entity, ui);
    //         }
    //     });
    // });

    let mut select = None;

    let quick_filter = search_popup(ui).unwrap_or_default();
    let height = ui.available_height();

    // let _focused_xxx_this_doesnt_work =
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            let mut quick_filter = quick_filter.clone();
            quick_filter.filter = false; //highlight, no point filtering parents.

            let header = draw_header(ui, "ancestors");
            for b in parent_chain.iter() {
                let mut filtered = false;
                let resp = draw_bundle(
                    &world,
                    b,
                    entity,
                    selected_c,
                    &quick_filter,
                    false,
                    ui,
                    &mut filtered,
                );
                if resp.inner.is_some() {
                    select = resp.inner;
                }
            }

            // NOTE: This was a good idea, but in the case of a really long name, it results in two wide columns instead of one
            // likewise, this leads to more layout change when switching through children.
            // draw_bundle(&world, &bundle, entity, ui);

            draw_line(ui, &header);
        });

        let mut ui_for = |ui: &mut Ui, siblings: &Vec<SmallVec<[Entity; 4]>>| {
            // TODO test
            ScrollArea::vertical()
                .id_salt(ui.next_auto_id())
                .min_scrolled_height(ui.available_height()) // XXX does this work?
                .show(ui, |ui| {
                    let mut filter_count = 0;
                    for b in siblings.iter() {
                        if b.contains(&entity) {
                            ui.scroll_to_cursor(Some(bevy_egui::egui::Align::Min));
                            //XXX borked
                        }

                        let mut filtered = false;
                        let resp = draw_bundle(
                            &world,
                            b,
                            entity,
                            selected_c,
                            &quick_filter,
                            true,
                            ui,
                            &mut filtered,
                        );
                        if resp.inner.is_some() {
                            select = resp.inner;
                        }

                        if filtered {
                            filter_count += 1;
                        } else if filter_count != 0 {
                            draw_ellipsis(ui, filter_count, false);
                            filter_count = 0;
                        }
                    }
                    if filter_count != 0 {
                        draw_ellipsis(ui, filter_count, false);
                    }
                });
        };

        ui.vertical(|ui| {
            ui.set_height(height);
            //TODO abstract to function
            let header = draw_header(ui, "siblings");

            // draw in order from start. but must included selected before running out of space.
            // and ideally works synergistically with additional sections below
            // 1. sections below could reflow to next pane, and side panel when that is out of space
            // 2. sections below could reflow to tabs
            ui_for(ui, &siblings);

            draw_line(ui, &header);
        });

        ui.vertical(|ui| {
            let header = draw_header(ui, "children");
            ui_for(ui, &children_bundled);
            // for b in children_bundled.iter() {
            //     let mut filtered = false;
            //     let resp = draw_bundle(
            //         &world,
            //         b,
            //         entity,
            //         selected_c,
            //         &quick_filter,
            //         true,
            //         ui,
            //         &mut filtered,
            //     );
            //     if resp.inner.is_some() {
            //         select = resp.inner;
            //     }
            // }
            draw_line(ui, &header);
        });
    });

    // remember selection for child
    if let Some(parent) = parent {
        let id = ui.id().with("selection").with(parent);
        let selected_child = entity;
        ui.memory_mut(|mem| mem.data.insert_temp::<Entity>(id, selected_child));
    }

    if focused {
        #[derive(Debug)]
        enum ExplorerAction {
            SelectParent,
            SelectChild,
            NextBundle,
            PrevBundle,
            NextSibling,
            PrevSibling,
            Enter,
            None,
        }

        // Check keybinds and return action
        let action = if quick_filter.enabled {
            // NOTE: unfortianately the textbox having focus does not prevent the keybinds from registering.
            ExplorerAction::None
        } else {
            ui.ctx().input_mut(|i| {
                if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::H)) {
                    ExplorerAction::SelectParent
                } else if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::L)) {
                    ExplorerAction::SelectChild
                } else if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::SHIFT, Key::J)) {
                    ExplorerAction::NextBundle
                } else if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::SHIFT, Key::K)) {
                    ExplorerAction::PrevBundle
                } else if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::J)) {
                    ExplorerAction::NextSibling
                } else if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::K)) {
                    ExplorerAction::PrevSibling
                } else if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::Enter)) {
                    ExplorerAction::Enter
                } else {
                    ExplorerAction::None
                }
            })
        };

        // Helper for filtering
        let qq = |entity: &Entity| !quick_filter.enabled || !quick_filter.filtered(&world, *entity);

        // Handle actions
        match action {
            ExplorerAction::SelectParent => {
                // Select parent, filtered if quick_filter is enabled
                if !quick_filter.enabled {
                    select = world.get::<ChildOf>(entity).map(|c| c.0);
                } else {
                    let mut iter = parent_chain
                        .iter()
                        .rev()
                        .flat_map(|b| b.iter().rev())
                        .cloned();
                    if let Some(e) = iter.find(qq) {
                        select = Some(e);
                    }
                }
            }
            ExplorerAction::SelectChild => {
                if let Some(children) = world.get::<Children>(entity) {
                    // Grab the last child selected for this entity.
                    // TODO would be nice if I didn't have to rely on these ids being hardcoded the same in two places
                    // WARNING XXX: memory mut with DEADLOCK if called inside input_mut
                    let id = ui.id().with("selection").with(entity);
                    let last_selected = ui.memory_mut(|mem| mem.data.get_temp::<Entity>(id));

                    dbg!(last_selected);
                    'skip: {
                        if let Some(last_selected) = last_selected {
                            let mut valid = children.iter().find(|c| *c == last_selected).is_some();
                            valid &= qq(&last_selected);
                            if valid {
                                select = Some(last_selected);
                                break 'skip;
                            }
                        }
                        if let Some(e) = children.iter().filter(qq).next() {
                            select = Some(e);
                        }
                    }
                }
            }
            ExplorerAction::NextBundle => {
                if bundle_index < bundle.len() - 1 {
                    select = Some(bundle[bundle_index + 1]);
                } else {
                    let i = (index + 1) % siblings.len();
                    select = Some(*siblings[i].first().unwrap())
                }
            }
            ExplorerAction::PrevBundle => {
                if bundle_index > 0 {
                    select = Some(bundle[bundle_index - 1]);
                } else {
                    let i = (index as isize - 1).rem_euclid(siblings.len() as isize) as usize;
                    select = Some(*siblings[i].last().unwrap());
                }
            }
            ExplorerAction::NextSibling => {
                for _ in 0..siblings.len() {
                    let i = (index + 1) % siblings.len();
                    if let Some(e) = siblings[i].iter().cloned().filter(qq).next() {
                        select = Some(e)
                    }
                }
            }
            ExplorerAction::Enter => {
                // THIS IS HOW QUICK MENU ACTUALLY IS USED. other keybinds broken
                // code can be deleted from other handlers, but maybe wat to adapt it to more general filtered nav mechanism in the future.
                // this should still trigger even though enter is what ends text focus
                let mut quick_filter = quick_filter.clone();
                quick_filter.enabled = true; //this get's set to false, buffer should still exist though
                dbg!(&quick_filter);
                let qq = |entity: &Entity| !quick_filter.filtered(&world, *entity);

                let sibs = siblings.iter().flat_map(|a| a.iter());
                let pars = parent_chain.iter().flat_map(|a| a.iter());
                let chld = children_bundled.iter().flat_map(|a| a.iter());
                if let Some(new_s) = sibs.chain(chld).chain(pars).cloned().filter(qq).next() {
                    select = Some(new_s);
                    dbg!(new_s);
                }
            }
            ExplorerAction::PrevSibling => {
                for _ in 0..siblings.len() {
                    let i = (index as isize - 1).rem_euclid(siblings.len() as isize) as usize;
                    if let Some(e) = siblings[i].iter().cloned().filter(qq).next() {
                        select = Some(e)
                    }
                }
            }
            ExplorerAction::None => {}
        }
    }

    select
}

#[derive(Debug, Clone, Default)]
struct SearchMode {
    enabled: bool,
    filter: bool,
    buffer: String, // TODO don't clone?
    case_insensitive: bool,
}

impl SearchMode {
    /// TODO reuse lookups
    fn filtered(&self, world: &World, entity: Entity) -> bool {
        let mut name = guess_entity_name(world, entity);
        if self.case_insensitive {
            name = name.to_ascii_lowercase();
        }

        !name.matches(&self.buffer).next().is_some()
            && !entity.to_string().matches(&self.buffer).next().is_some()
    }
}

fn search_popup(ui: &mut Ui) -> Option<SearchMode> {
    let id = ui.id().with("search popup");
    let mut mode = ui.memory_mut(|m| m.data.get_temp_mut_or_default::<SearchMode>(id).clone());

    if !mode.enabled {
        let (search, filter) = ui.ctx().input_mut(|i| {
            (
                i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::Slash)),
                i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::Questionmark)),
            )
        });
        if search || filter {
            ui.memory_mut(|m| {
                let mode: &mut SearchMode = m.data.get_temp_mut_or_default(id);
                mode.enabled = true;
                mode.filter = filter;
                mode.buffer.clear();
            });
        }

        None
    } else {
        let layer = LayerId {
            order: egui::Order::Foreground,
            id,
        };
        let mut child = ui.new_child(
            UiBuilder::new()
                .layer_id(layer)
                .max_rect(ui.clip_rect())
                .layout(Layout::bottom_up(egui::Align::Center)),
        );
        let resp = child.add(TextEdit::singleline(&mut mode.buffer));
        if resp.lost_focus() {
            // close
            mode.enabled = false;
        } else if !resp.has_focus() {
            // quick search is either focused or closes
            resp.request_focus();
        }

        // persist changes
        ui.memory_mut(|m| {
            m.data.insert_temp::<SearchMode>(id, mode.clone());
        });

        mode.case_insensitive = mode.buffer.to_ascii_lowercase() == mode.buffer;

        Some(mode)
    }
}

fn _search_prompt(ui: &mut Ui, text: Option<&mut String>) -> String {
    // Defining these up here is cumbersome...
    let memory_text_fallback;
    let mut guard;
    let buffer = if text.is_none() {
        memory_text_fallback = ui.memory_mut(|mem| {
            // this data is only used singlethreaded so it should work with refcell?
            mem.data
                .get_temp_mut_or_default::<Arc<Mutex<String>>>(ui.next_auto_id())
                .clone()
        });
        guard = memory_text_fallback.try_lock().unwrap();
        guard.deref_mut()
    } else {
        text.unwrap()
    };

    TextEdit::singleline(buffer).show(ui);

    buffer.clone()
}

fn draw_header(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    let text = text.into().small_raised();
    ui.label(text)
}

/// draws the line for the header, has to be done at end since that's when we know min_size
/// TODO: could this be done some more ideomatic way
/// TODO: could this be a drop impl?
fn draw_line(ui: &mut Ui, resp: &Response) {
    let y = resp.rect.y_range().center();
    let x1 = resp.rect.x_range().max;
    let x2 = ui.min_rect().x_range().max;
    ui.painter().line_segment(
        [Pos2::new(x1, y), Pos2::new(x2, y)],
        Stroke::new(1.0, Color32::from_gray(100)),
    );
}

fn draw_bundle(
    world: &World,
    entity: &EntityBundle,
    selected: Entity,
    selected_c: &Vec<ComponentId>,
    quick_filter: &SearchMode, // XXX would be better to batch fetch Name, right? use queries?
    horizontal: bool,
    ui: &mut Ui,
    filtered_bundle_ret: &mut bool,
) -> bevy_egui::egui::InnerResponse<Option<Entity>> {
    let bundle_selected = entity.contains(&selected);
    let mut resp = Frame::new();

    let color = match bundle_selected {
        true => Color32::from_gray(0xaa),
        false => Color32::BLACK,
    };

    let max_width = match horizontal {
        true => 15,
        false => 0, // unlimited
    };

    if entity.len() > 1 && !horizontal {
        resp = resp
            .corner_radius(1.0)
            .stroke(bevy_egui::egui::Stroke { width: 1.0, color });
        // if bundle_selected {
        //     // resp = resp.fill(Color32::from_rgba_premultiplied(u8::MAX, u8::MAX, u8::MAX, 10))
        //     resp = resp.corner_radius(1.0).stroke(bevy_egui::egui::Stroke {
        //         width: 1.0,
        //         color: Color32::from_gray(0xaa),
        //     });
        // };
    };

    resp.show(ui, |ui| {
        //TODO better prop
        let mut select = None;

        let layout = match horizontal {
            false => Layout::top_down(bevy_egui::egui::Align::Min),
            true => Layout::left_to_right(bevy_egui::egui::Align::Min),
        };

        // This deals with indicating filtered parents / siblings
        let mut drawn_one = false;
        let mut filter_count = 0;

        ui.with_layout(layout, |ui| {
            let mut first = true;
            for e in entity.iter() {
                let mut filtered = false;
                if quick_filter.enabled {
                    // filtered = quick_filter.filtered(world, *e) && selected != *e; // don't filter selected element
                    filtered = quick_filter.filtered(world, *e); // actually do filter selected element
                    if quick_filter.filter {
                        if filtered {
                            filter_count += 1;
                            continue;
                        } else if filter_count != 0 {
                            // we skipped some values, draw the ellipsis
                            draw_ellipsis(ui, filter_count, horizontal);
                            filter_count = 0;

                            // yup, this logic unfortionatley interacts.
                            first = false;
                        }
                    }
                }

                if horizontal && !first {
                    // ui.label(RichText::from("❱").monospace());
                    ui.label(
                        RichText::from("/")
                            .monospace()
                            .color(Color32::from_white_alpha(0x22)),
                    ); // TODO CONST colors
                }
                first = false;

                drawn_one = true;
                if draw_entity(
                    world,
                    *e,
                    selected,
                    selected_c,
                    quick_filter,
                    filtered,
                    max_width,
                    ui,
                ) {
                    // TODO refactor hacky selection code path
                    select = Some(*e);
                }
            }
            if filter_count != 0 && drawn_one {
                draw_ellipsis(ui, filter_count, horizontal);
            }
            *filtered_bundle_ret = !drawn_one;
        });
        select
    })
}

fn draw_ellipsis(ui: &mut Ui, count: usize, horizontal: bool) {
    match horizontal {
        true => {
            // Don't display 1's, they were really unnecessary and distracting.
            let text = RichText::new("…").weak().monospace();
            ui.label(text);
            if count > 1 {
                let text = format!("{}", count);
                let text = RichText::new(text).weak().small_raised().monospace();
                ui.label(text);
            }
        }
        false => {
            // TODO thin line with number
            let text = match count > 1 {
                true => format!("...{}", count),
                false => "...".into(),
            };
            let text = RichText::new(text).small().monospace();
            ui.label(text);
        }
    }
}
/// for now: Entity, Name or standin
/// todo: color (red for mesh, grey for observers, blue for scenes)
/// todo: sigils (networking, asset loading)
/// todo: special marker for root node / leaf node
/// The following need context info to turn on/off
/// todo: how many children
/// todo: how many parents to root?
/// todo: sigil for selected component is default
fn draw_entity(
    world: &World,
    entity: Entity,
    selected: Entity,
    selected_c: &Vec<ComponentId>, 
    quick_filter: &SearchMode,
    filtered: bool,
    max_width: u16,
    ui: &mut Ui,
) -> bool {
    let is_selected = entity == selected;

    // Try to get the Name component, fallback to entity id
    let mut name = guess_entity_name(world, entity);

    if max_width != 0 && (max_width as usize) < name.len() {
        // NOTE: don't do this because lots of long names should have entity ids line up.
        // let offset = entity.to_string().len();
        // max_width.saturating_sub(offset).max(3);

        let elipsis = "…";
        name = format!("{name:.*}{elipsis}", max_width as usize - elipsis.len());
    }

    // Simple color: highlight if selected
    // TODO mesh = red
    let text_color = color_setting(is_selected);

    // sigil rendering hook.
    // TODO

    // Draw the entity label
    let frame = Frame::new();
    let resp = frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            // Check if entity has Name component
            let has_name = world.get::<Name>(entity).is_some();
            let mut name_text = if has_name {
                bevy_egui::egui::RichText::new(&name).color(text_color)
            } else {
                bevy_egui::egui::RichText::new(&name)
                    .color(text_color)
                    .italics()
            };

            if !selected_c.is_empty() {
                let e = world.entity(entity);
                if selected_c.iter().all(|c| !e.contains_id(*c)) {
                    //name_text = name_text.weak(); // doesn't seem to do anything.
                    if !is_selected {
                        name_text = bevy_egui::egui::RichText::new(&name)
                            .color(text_color.gamma_multiply(0.3))
                    }
                } else {
                    name_text = name_text.strong();
                }
            }

            if quick_filter.enabled && filtered {
                name_text = name_text.weak().strikethrough().color(Color32::BLACK)
            }
            // TODO decide proper styling
            // if quick_filter.enabled && !quick_filter.filter && !quick_filter.filtered {
            //     name_text = name_text.strong()
            // }

            let mut layout_job = LayoutJob::default();
            name_text.append_to(
                &mut layout_job,
                ui.style(),
                egui::FontSelection::Default,
                ui.layout().vertical_align(),
            );

            if quick_filter.enabled && !filtered {
                let m = highlight_match(
                    &mut layout_job,
                    &quick_filter.buffer,
                    quick_filter.case_insensitive,
                );

                // NOTE: hmm, need to have access to matching in the keyboard nav code.
                // if !m {
                //     match quick_filter.filter {
                //         true => {
                //             layout_job =
                //         },
                //         false => todo!(),
                //     }
                // }
            }

            ui.label(layout_job);
            ui.add_space(0.5);

            // Entity id in grey
            // philosophically inclined to minimum text, no need for [1v0] or #1v0, just `name 0v1` is fine
            let entity_text = bevy_egui::egui::RichText::new(format!("{}", entity.index()))
                .color(bevy_egui::egui::Color32::GRAY);
            let mut layout_job = LayoutJob::default();
            entity_text.append_to(
                &mut layout_job,
                ui.style(),
                egui::FontSelection::Default,
                ui.layout().vertical_align(),
            );
            if quick_filter.enabled && !filtered {
                let m = highlight_match(
                    &mut layout_job,
                    &quick_filter.buffer,
                    quick_filter.case_insensitive,
                );
            }
            ui.label(layout_job);

            let nchildren = world
                .get::<Children>(entity)
                .map(|cs| cs.len())
                .unwrap_or_default();
            if nchildren > 1 {
                ui.add_space(1.5);
                ui.label(
                    bevy_egui::egui::RichText::new(format!("[{}]", nchildren))
                        .color(bevy_egui::egui::Color32::GRAY)
                        .text_style(bevy_egui::egui::TextStyle::Small),
                );
            };
        });
    });

    //TODO highlight on hover
    let clicked = resp.response.interact(Sense::CLICK).clicked();
    clicked
}

fn color_setting(selected: bool) -> bevy_egui::egui::Color32 {
    // Simple color: highlight if selected
    if selected {
        bevy_egui::egui::Color32::LIGHT_BLUE
    } else {
        bevy_egui::egui::Color32::WHITE
    }
}

/// beautiful function which highlights matching text in a layoutjob   
fn highlight_match(layout_job: &mut LayoutJob, pattern: &str, case_insensitive: bool) -> bool {
    let mut ret = false;

    let text = if case_insensitive {
        layout_job.text.to_ascii_lowercase()
    } else {
        layout_job.text.clone()
    };

    // TODO this should not be two loops
    for range in text.match_indices(pattern).map(|(_, s)| {
        /* convert str to byte range */
        let start = s.as_ptr() as usize - text.as_ptr() as usize;
        // let start = unsafe{ s.as_ptr().offset_from_unsigned(text.text.as_ptr())}; // 85% sure this is sound.
        start..(start + s.as_bytes().len())
    }) {
        let mut ii = 0;
        while let Some(section) = layout_job.sections.get_mut(ii) {
            fn intersect(a: &Range<usize>, b: &Range<usize>) -> Option<Range<usize>> {
                let end = min(a.end, b.end);
                let start = max(a.start, b.start);
                if start < end {
                    Some(start..end)
                } else {
                    None
                }
            }

            if let Some(range) = intersect(&section.byte_range, &range) {
                // split up section ii
                let original = section.clone();
                section.byte_range = range.clone();

                // TODO config
                section.format.underline = Stroke::new(2.0, section.format.color);
                section.format.background = Color32::from_black_alpha(0x01);
                ret = true;

                if original.byte_range.start < range.start {
                    let mut original = original.clone();
                    original.byte_range.end = range.start;
                    layout_job.sections.insert(ii, original);
                    ii += 1;
                }
                if original.byte_range.end > range.end {
                    let mut original = original.clone();
                    original.byte_range.start = range.end;
                    layout_job.sections.insert(ii + 1, original);
                    ii += 1;
                }
            }
            ii += 1;
        }
    }

    ret
}

// fn find_relations(world : &World, reg: &TypeRegistry){
//     for c in world.components().iter_registered() {
//         let Some(typ) = c.type_id().and_then(|t| reg.get_type_info(t) ) else {
//             continue;
//         };

//     }
// }

type EntityBundle = SmallVec<[Entity; 4]>;

// struct EntityBundle {
//     entities: SmallVec<[Entity; 4]>,
//     parent: Option<Entity>,
// }

pub trait DynamicRelation {
    type Relationship: Relationship;

    fn parent(world: &World, entity: Entity) -> Option<Entity> {
        world.get::<Self::Relationship>(entity).map(|r| r.get())
    }
    fn children(world: &World, entity: Entity) -> Option<Vec<Entity>> {
        //type RelationshipTarget = <Self::Relationship as Relationship>::RelationshipTarget;
        world
            .get::<<Self::Relationship as Relationship>::RelationshipTarget>(entity)
            .map(|rt| rt.iter().collect())
    }
}

pub struct DynamicRelationMetadata {
    pub relationship: TypeId,
    pub relationship_target: TypeId,
}

#[reflect_trait]
pub trait DynamicRelationDumb {
    fn parent(&self, world: &mut World, entity: Entity) -> Option<Entity>;
    fn children(&self, world: &mut World, entity: Entity) -> Option<Vec<Entity>>;
    fn metadate(&self) -> DynamicRelationMetadata;
}

impl<T: DynamicRelation> DynamicRelationDumb for T {
    fn parent(&self, world: &mut World, entity: Entity) -> Option<Entity> {
        T::parent(world, entity)
    }

    fn children(&self, world: &mut World, entity: Entity) -> Option<Vec<Entity>> {
        T::children(world, entity)
    }

    fn metadate(&self) -> DynamicRelationMetadata {
        DynamicRelationMetadata {
            relationship: TypeId::of::<T::Relationship>(),
            relationship_target: TypeId::of::<<T::Relationship as Relationship>::RelationshipTarget>(
            ),
        }
    }
}

// impl RegistryHelper for &World {
//     fn relations(&self) -> impl Iterator<Item = ComponentId> {
//         self.components().iter_registered().filter(
//             |reg| reg.ty
//         )
//     }
// }

// There is no runtime data for relations.

/// examples:
/// physics:  RigidBody dark, Collider light, Sensor hollow circle.
/// networking: Authority filled circle, Replicated hollow circle.
/// animation: Player dark, target light, (hollow when not playing)
struct ComponentENavUiSettings {
    sigil: SystemId,
    highlight: Color,
}

const SIGILS: &str = "◉○◌◍◎●◐◑◒◓◔◕ ⊕⊖⊗⊘⊙⊚⊛⊜⊝ ◈◇◆ ✚";
