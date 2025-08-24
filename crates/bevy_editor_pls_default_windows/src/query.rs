// pieces needed for editor
// 1. dynamic querys
// 2. persistence
// 3. composability
// 4. command pallete

use std::{any::TypeId, ops::Rem, process::Child, u8};

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
    emath::Numeric,
    FontId,
    Frame,
    Key,
    KeyboardShortcut,
    Margin,
    Modifiers,
    Response,
    RichText,
    Sense,
    Stroke,
    Ui,
    Vec2,
    WidgetText,
};
use bevy_inspector_egui::bevy_inspector::guess_entity_name;
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

    use crate::{hierarchy::HierarchyState, query::draw_explorer};

    #[derive(Debug, Default, Component, Clone, Copy)]
    pub struct NavWindow;
    impl EditorWindow for NavWindow {
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

            let entities = cx.get::<HierarchyState>(world).unwrap().selected.clone();

            if let Some(e) = entities.iter().next() {
                if let Some(e) = draw_explorer(world, e, ui) {
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

fn draw_explorer(world: &mut World, entity: Entity, ui: &mut Ui) -> Option<Entity> {
    let registry = world.resource::<AppTypeRegistry>().clone();

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
    let siblings: Vec<EntityBundle> = match parent {
        Some(parent) => {
            let children = world.entity(parent).get::<Children>().unwrap();
            let children = children // gets rid of world borrow
                .into_iter()
                .cloned()
                .collect::<SmallVec<[Entity; 16]>>();

            index = children
                .iter()
                .enumerate()
                .find(|a| *a.1 == bundle[0])
                .unwrap()
                .0; //TODO cleaner

            children
                .into_iter()
                .map(|e| world.run_system_cached_with(get_bundle, e).unwrap())
                .collect()
        }
        None => vec![bundle.clone()],
    };

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

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            let header = draw_header(ui, "ancestors");
            for b in parent_chain {
                let resp = draw_bundle(&world, &b, entity, ui);
                if resp.inner.is_some() {
                    select = resp.inner;
                }
            }

            // NOTE: This was a good idea, but in the case of a really long name, it results in two wide columns instead of one
            // likewise, this leads to more layout change when switching through children.
            // draw_bundle(&world, &bundle, entity, ui);

            draw_line(ui, &header);
        });
        ui.vertical(|ui| {
            //TODO abstract to function
            let header = draw_header(ui, "siblings");

            // draw in order from start. but must included selected before running out of space.
            // and ideally works synergistically with additional sections below
            // 1. sections below could reflow to next pane, and side panel when that is out of space
            // 2. sections below could reflow to tabs
            for b in siblings.iter() {
                let resp = draw_bundle(&world, b, entity, ui);
                if resp.inner.is_some() {
                    select = resp.inner;
                }
            }

            draw_line(ui, &header);
        });
        ui.vertical(|ui| {
            let header = draw_header(ui, "children");
            for b in children_bundled.iter() {
                let resp = draw_bundle(&world, b, entity, ui);
                if resp.inner.is_some() {
                    select = resp.inner;
                }
            }
            draw_line(ui, &header);
        });
    });

    ui.ctx().input_mut(|i| {
        if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::H)) {
            select = world.get::<ChildOf>(entity).map(|c| c.0);
        }
        if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::L)) {
            // TODO restore selection
            select = world.get::<Children>(entity).and_then(|c| c.iter().next());
        }
        if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::SHIFT, Key::J)) {
            if bundle_index < bundle.len() - 1 {
                select = Some(bundle[bundle_index + 1]);
            } else {
                let i = (index + 1) % siblings.len();
                select = Some(*siblings[i].first().unwrap())
            }
        }
        if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::SHIFT, Key::K)) {
            if bundle_index > 0 {
                select = Some(bundle[bundle_index - 1]);
            } else {
                let i = (index as isize - 1).rem_euclid(siblings.len() as isize) as usize;
                select = Some(*siblings[i].last().unwrap());
            }
        }
        if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::J)) {
            let i = (index + 1) % siblings.len();
            select = Some(*siblings[i].first().unwrap())
        }
        if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::K)) {
            let i = (index as isize - 1).rem_euclid(siblings.len() as isize) as usize;
            select = Some(*siblings[i].first().unwrap())
        }
    });

    select
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
    ui: &mut Ui,
) -> bevy_egui::egui::InnerResponse<Option<Entity>> {
    let bundle_selected = entity.contains(&selected);
    let mut resp = Frame::new();

    if entity.len() > 1 {
        resp = resp.corner_radius(1.0).stroke(bevy_egui::egui::Stroke {
            width: 1.0,
            color: Color32::BLACK,
        });
        if bundle_selected {
            // resp = resp.fill(Color32::from_rgba_premultiplied(u8::MAX, u8::MAX, u8::MAX, 10))
            resp = resp.corner_radius(1.0).stroke(bevy_egui::egui::Stroke {
                width: 1.0,
                color: Color32::from_gray(0xaa),
            });
        };
    };

    resp.show(ui, |ui| {
        //TODO better prop
        let mut select = None;
        ui.vertical(|ui| {
            for e in entity {
                if draw_entity(world, *e, selected, ui) {
                    select = Some(*e);
                }
            }
        });
        select
    })
}

/// for now: Entity, Name or standin
/// todo: color (red for mesh, grey for observers, blue for scenes)
/// todo: sigils (networking, asset loading)
/// todo: special marker for root node / leaf node
/// The following need context info to turn on/off
/// todo: how many children
/// todo: how many parents to root?
fn draw_entity(world: &World, entity: Entity, selected: Entity, ui: &mut Ui) -> bool {
    let is_selected = entity == selected;

    // Try to get the Name component, fallback to entity id
    let name = guess_entity_name(world, entity);

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
            let name_text = if has_name {
                bevy_egui::egui::RichText::new(name).color(text_color)
            } else {
                bevy_egui::egui::RichText::new(name)
                    .color(text_color)
                    .italics()
            };
            ui.label(name_text);
            ui.add_space(0.5); // philosophically inclined to minimum text, no need for [1v0] or #1v0, just `name 0v1` is fine

            // Entity id in grey
            ui.label(
                bevy_egui::egui::RichText::new(format!("{}", entity.index()))
                    .color(bevy_egui::egui::Color32::GRAY),
            );

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
