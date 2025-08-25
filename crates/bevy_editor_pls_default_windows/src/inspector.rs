use std::any::{Any, TypeId};
use std::cell::UnsafeCell;
use std::collections::{hash_set, BTreeMap};
use std::panic::Location;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::hierarchy::HierarchyState;
use crate::utils::open::{open_file_at_line, open_location};

use bevy::diagnostic::FrameCount;
// use super::add::{AddWindow, AddWindowState};
use bevy::prelude::*;
use bevy::app::Plugin;
use bevy::asset::UntypedAssetId;
use bevy::color::palettes::css::LIGHT_GREY;
use bevy::ecs::bundle;
use bevy::ecs::change_detection::DetectChangesMut;
use bevy::ecs::component::{Component, ComponentId, ComponentInfo, ComponentTicks, Components, Tick};
use bevy::ecs::entity::Entity;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::resource::Resource;
use bevy::ecs::world::DynamicComponentFetch;
use bevy::log::error_once;
use bevy::log::tracing_subscriber::fmt::format;
use bevy::log::tracing_subscriber::registry;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::{AppTypeRegistry, World};
use bevy::reflect::prelude::ReflectDefault;
use bevy::reflect::{Reflect, TypeInfo, TypePath, TypeRegistry};
use bevy::render::RenderApp;
use bevy::state::reflect;
use bevy::time::TimeSystem;
use bevy::transform::components;
use bevy::utils::default;
use bevy_editor_pls_core::editor_window::{DefaultLink, EditorWindow, EditorWindowContext, Link};
use bevy_editor_pls_core::AddEditorWindow;
use bevy_egui::egui::{lerp, CollapsingHeader, RichText, Sense};
use bevy_inspector_egui::bevy_inspector::hierarchy::SelectedEntities;
use bevy_inspector_egui::egui_utils::easymark::viewer::easy_mark;
use bevy_inspector_egui::reflect_inspector::{ui_for_value, InspectorUi};
use bevy_inspector_egui::restricted_world_view::{RestrictedWorldView};
use bevy_inspector_egui::{bevy_inspector, egui};
use bevy_metrics_dashboard::metrics::Key;
use smallvec::SmallVec;
use tracing_core::callsite::register;
use tracing_log::log::info;
use transform_gizmo_bevy::Color32;

// TODO cant make reflect because of UnTypedAssetId
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub enum InspectorSelection {
    #[default]
    Entities,
    Resource(ComponentId, String),
    Asset(TypeId, String, UntypedAssetId),
}

#[derive(Debug, Clone, Default, Component, Reflect, PartialEq)]
pub struct InspectorState {
    #[reflect(ignore)]
    pub selected: InspectorSelection,
    pub component_selected: Vec<ComponentId>,

    // TODO for restoring state
    pub changed: bool, 
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

        let mut selected = cx.get::<InspectorState>(world).unwrap().clone(); // TODO don't clone
        let entities = &cx.get::<HierarchyState>(world).unwrap().selected.clone();

        // let add_window_state = cx.state::<AddWindow>();
        inspector(
            world,
            &mut selected,
            entities,
            ui,
            // add_window_state,
            &type_registry,
        );

        // TODO change detect fix
        *cx.get_mut::<InspectorState>(world).unwrap() = selected;
    }
}

impl Plugin for InspectorWindow {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_editor_window::<Self>();
        app.register_type::<InspectorState>();
        app.register_type::<Link<InspectorState>>();
        app.register_type::<DefaultLink<InspectorState>>();
        app.init_resource::<DefaultLink<InspectorState>>();

        app.register_type::<TicksTimer>();
        app.init_resource::<TicksTimer>();
        app.add_systems(First, TicksTimer::update_system.after(TimeSystem));
    }
}

fn inspector(
    world: &mut World,
    selected: &mut InspectorState,
    selected_entities: &SelectedEntities,
    ui: &mut egui::Ui,
    // add_window_state: Option<&AddWindowState>,
    type_registry: &TypeRegistry,
) {
    egui::ScrollArea::vertical().show(ui, |ui| match selected.selected {
        InspectorSelection::Entities => match selected_entities.as_slice() {
            [] => {
                ui.label("No entity selected");
            }
            &[entity] => {
                // bevy_inspector::ui_for_entity(world, entity, ui);
                // add_ui(ui, &[entity], world, add_window_state);
                ui.horizontal(|ui| {
                    new_inspector(
                        world,
                        entity,
                        ui,
                        type_registry,
                        &mut selected.component_selected,
                    );

                    ui.vertical(|ui| {
                        for c in selected.component_selected.iter() {
                            let component =
                                CompInfo::try_from_world(&world, *c, type_registry, None).unwrap();
                            ui_for_entity_component(world, component, entity, ui, type_registry);
                        }
                    });

                });
            }
            entities => {
                bevy_inspector::ui_for_entities_shared_components(world, entities, ui);
                // add_ui(ui, entities, world, add_window_state);
            }
        },
        InspectorSelection::Resource(id, ref name) => {
            ui.label(name);
            let Some(info) = world.components().get_info(id) else {
                return;
            };
            let Some(type_id) = info.type_id() else {
                return;
            };
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

#[derive(Debug, Default, Clone, Copy)]
struct TicksInfo {
    ticks: Option<ComponentTicks>,
    changed_by: Option<&'static Location<'static>>
}

impl TicksInfo {
    /// function to get changed_by and ticks by id, 
    /// seems bevy doesn't have a nice function to do this for changed_by yet
    /// SOMEDAY: deleteme
    fn get(world: &World, entity: Entity, component: ComponentId) -> Self{
        // TODO move to function
        let entity_loc = world.entities().get(entity).unwrap();
        let arch = world.archetypes().get(entity_loc.archetype_id).unwrap();

        // NOTE this exists but nothing for changed by
        // world.entity(entity).get_change_ticks_by_id(component);

        let mut ret = Self::default();

        match arch.get_storage_type(component) {
            None => {}, // Compoenent missing
            Some(bevy::ecs::component::StorageType::Table) => {
                let table = world.storages().tables.get(entity_loc.table_id).unwrap();
                assert!(table.capacity() > entity_loc.table_row.as_usize()); //SAFETY
                ret.ticks = unsafe { table.get_ticks_unchecked(component, entity_loc.table_row) };
                
                let by = table.get_changed_by(component, entity_loc.table_row).into_option().flatten();
                ret.changed_by = by.map(|cell| unsafe {*cell.get()});
            },
            Some(bevy::ecs::component::StorageType::SparseSet) => {
                let set = world.storages().sparse_sets.get(component).unwrap();
                ret.ticks = set.get_ticks(entity);

                let by = set.get_changed_by(entity).into_option().flatten();
                ret.changed_by = by.map(|cell| unsafe {*cell.get()});
            }
        };

        ret
    }

    fn added(&self, world: &mut World) -> Option<bool>{
        let ticks = self.ticks?;
        let added = ticks.added.is_newer_than(world.last_change_tick(), world.change_tick());
        Some(added)
    }

    fn changed(&self, world: &mut World) -> Option<bool>{
        let ticks = self.ticks?;
        let changed = ticks.changed.is_newer_than(world.last_change_tick(), world.change_tick());
        Some(changed)
    }
    
    fn changed_time(&self, world: &World) -> Option<Duration>{
        let ticks = self.ticks?;
        let history = world.get_resource::<TicksTimer>()?;
        Some(history.now()?.elapsed - history.get_time(ticks.changed)?.elapsed)
    }
    
    fn added_time(&self, world: &World) -> Option<Duration>{
        let ticks = self.ticks?;
        let history = world.get_resource::<TicksTimer>()?;
        Some(history.now()?.elapsed - history.get_time(ticks.added)?.elapsed)
    }
}

// #[derive(Debug, Clone, Copy, Eq, PartialEq)]
// struct OrderWrappedU32(pub u32);

// impl Ord for OrderWrappedU32 {
//     fn cmp(&self, other: &Self) -> std::cmp::Ordering {
//         let self_to_other = self.0.wrapping_sub(other.0);
//         let other_to_self = other.0.wrapping_sub(self.0);

//         self_to_other.cmp(&other_to_self)
//     }
// }

// impl PartialOrd for OrderWrappedU32 {
//     fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
//         Some(self.0.cmp(&other.0))
//     }
// }

// #[test]
// mod test {
//     use std::{cmp::Ordering, u32};

//     use super::OrderWrappedU32;
//     fn test_order_wrapped_u32(){
//         let a = OrderWrappedU32(56);
//         for b in 0..55 {
//             assert!(OrderWrappedU32(b) < a);
//         }
//         assert_eq!(OrderWrappedU32(56).cmp(a), Ordering::Equal);

//         for b in 57..100 {
//             assert!(OrderWrappedU32(b) > a);
//         }

//         // THE PROBLEM
//         let center = u32::MAX / 2 + 56;
//         assert_eq!(OrderWrappedU32(center).cmp(a), Ordering::Equal);

//         for b in (center-100)..center {
//             assert!(OrderWrappedU32(b) > a);
//         }
        
//         for b in (center+1)..(center+100) {
//             assert!(OrderWrappedU32(b) < a);
//         }

//         for b in (u32::MAX-100)..u32::MAX {
//             assert!(OrderWrappedU32(b) < a);
//         }
//         assert!(OrderWrappedU32(u32::MAX) < a);
//     }
// }


#[derive(Debug, Clone, Default, Resource, Reflect)]
struct TicksTimer{
    history: BTreeMap<u64, TickMetadata>,
    wrappings: u32,
}

/// TODO frame should have enum for accuracy
/// this should have a custom debug impl
#[derive(Debug, Copy, Clone, Default, Reflect)]
struct TickMetadata{
    tick: Tick,
    elapsed: Duration,
    frame: u32,
}

impl TicksTimer {
    // should run First after TimeSystem
    fn update_system(time: Res<Time<Virtual>>, mut history: ResMut<Self>, frame: Res<FrameCount>){
        // let tick = OrderWrappedU32(history.last_changed().get());
        let tick = history.last_changed();
        let tick_u32 = tick.get();
        let last = history.history.last_key_value().map(|a|*a.0).unwrap_or_default();
        if tick_u32 < last as u32 {
            //u32 wrap
            history.wrappings += 1;
        }
        let tick_u64 = tick_u32 as u64 | (history.wrappings as u64) << 32;
        let time = time.elapsed();

        #[cfg(debug_assertions)]
        if let Some((last_tick, last_time)) = history.history.last_key_value() {
            assert!(tick_u64 > *last_tick);
            assert!(time > last_time.elapsed);
            assert!(frame.0 > last_time.frame);
        }
        
        history.history.insert(tick_u64, TickMetadata { tick, elapsed: time, frame: frame.0 });
        history.filter_old();
    }

    fn filter_old(&mut self){
        let first = self.history.first_key_value().unwrap();
        let mut prev_time = first.1.elapsed;
        let first_tick = *first.0;

        let now = self.history.last_key_value().unwrap().1.elapsed;
        self.history.retain(|&tick, &mut data| {
            assert!(first_tick <= tick);
            assert!(prev_time <= data.elapsed);
            assert!(now >= data.elapsed);

            if tick == first_tick {
                return true;
            }
            // just use u64
            // if tick.0.wrapping_sub(&k.0) >= u32::MAX / 2 {
            //     // auto wrapping btree can't handle using more than half the int range.
            //     return false;
            // }

            // 0.5%
            let resolution = (now - data.elapsed).div_f32(200.0);
            let to_remove = (data.elapsed - prev_time) < resolution;
            if to_remove {
                return false;
            }else{
                prev_time = data.elapsed;
                return true;
            }
        });

        //dbg!(self.history.len());
    }

    /// returns duration since app start associated with the tick.
    fn get_time(&self, tick2: Tick) -> Option<TickMetadata> {
        let tick = tick2.get() as u64 | ((self.wrappings as u64) << 32);

        let Some(b) = self.history.range(tick..).next() else {
            // tick is newer than start of frame
            return self.now();
        };

        let Some(a) = self.history.range(..=tick).next_back() else {
            // tick is older than first frame, weird.
            return None;
        };

        if tick == *a.0 {
            return Some(*a.1)
        }
        
        let factor = (tick - a.0) as f32 / (b.0 - a.0) as f32;
        let lerp = |a, b:Duration, f| ( b - a ).mul_f32(f) + a; 
        let lerp2 = |a, b, f| ((b as f32 - a as f32) * f + a as f32) as u32; 
        Some(TickMetadata { tick: tick2, elapsed: lerp(a.1.elapsed, b.1.elapsed, factor), frame: lerp2(a.1.frame, b.1.frame, factor) })
    }

    fn now(&self) -> Option<TickMetadata> {
        Some(*self.history.last_key_value()?.1)
    }
}

fn new_inspector(
    world: &mut World,
    entity: Entity,
    ui: &mut egui::Ui,
    // add_window_state: Option<&AddWindowState>,
    type_registry: &TypeRegistry,
    selected: &mut Vec<ComponentId>,
) {
    let plan = ComponentPlan::bundle_components(world, entity, type_registry);

    ui.vertical(|ui| {
        // let mut drawn = HashSet::with_capacity(ps.len());
        for crate_name in plan.paths() {
            let path_set = plan.path.get(&crate_name).unwrap();
            let bundles = plan.bundles_for_path(&crate_name).unwrap();

            egui::CollapsingHeader::new(crate_name).show(ui, |ui| {
                for bundle in bundles.iter() {
                    let bundle_selected = bundle.iter().any(|id| selected.contains(id));

                    // todo make function
                    let mut resp = egui::Frame::new();
                    if bundle.len() > 1 {
                        resp = resp.corner_radius(1.0).stroke(bevy_egui::egui::Stroke {
                            width: 1.0,
                            color: egui::Color32::BLACK,
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
                        for id in bundle.iter() {
                            let info = plan.info.get(id).unwrap();
                            let in_bundle = path_set.contains(id);
                            let active = world.entity(entity).contains_id(*id);

                            let ticks = TicksInfo::get(&world, entity, *id);
                            let added = ticks.added(world);
                            let changed = ticks.changed(world);
                            
                            let reflect = info
                                .info
                                .type_id()
                                .and_then(|id| world.get_reflect(entity, id).ok());
                            let reflect_component = reflect.and_then(|r| type_registry.get_type_data::<ReflectComponent>(r.reflect_type_info().type_id()));
                            let is_default = reflect
                                .and_then(|r| reflect_is_default(r.as_reflect(), type_registry));

                            // TODO visual indication of required_component arrows
                            ui.horizontal(|ui| {
                                // change detection
                                let mut text = match changed {
                                    Some(true)  => "◇",
                                    Some(false) => " ",
                                    None        => match active {
                                        true => "?",
                                        false => " "
                                    },
                                };
                                if added.unwrap_or_default(){
                                    text = "◈";
                                }
                                
                                let text : RichText = text.into();
                                ui.label(text.monospace().color(Color32::GRAY));

                                // primary label (name)
                                let mut text = RichText::new(info.short_name());
                                if ! in_bundle {
                                    text = text.weak();
                                }
                                if selected.contains(id) {
                                    text = text.color(Color32::LIGHT_BLUE);
                                }
                                if reflect.is_none() {
                                    // not reflect
                                    text = text.italics();
                                }
                                if !active {
                                    // missing required component
                                    text = text.strikethrough();
                                }

                                if ui
                                    .label(text)
                                    .interact(Sense::click())
                                    .clicked()
                                {
                                    if !ui.input(|i| i.modifiers.shift) {
                                        selected.clear();
                                        selected.push(*id);
                                    }else{
                                        if let Some(i) = selected.iter().position(|s| s == id){
                                            selected.remove(i);
                                        }else{
                                            selected.push(*id);
                                        }
                                    }
                                }
                                
                                // warning sigil
                                let text = RichText::from("⚠").color(Color32::YELLOW);
                                if reflect.is_some() && reflect_component.is_none(){
                                    let err = format!("{} has Reflect but not ReflectComponent\nit is likely missing `#![reflect(Component)]` on struct definition", info.short_name() );
                                    ui.label(text.clone()).on_hover_ui( |ui| easy_mark(ui, &err)); // TODO regularized error messages.
                                }
                                if info.type_info.is_none(){
                                    let err = 
                                        format!("{} has no type registration\nit is likely missing `app.register_type::<TYPE>()` and/or `#![derive(Reflect)]`", info.short_name() );
                                    ui.label(text).on_hover_ui(|ui| easy_mark(ui, &err)); // TODO regularized error messages.
                                }

                                // is_default sigil
                                match is_default {
                                    Some(true) => {
                                        ui.label(RichText::new("D").small_raised().weak());
                                    }
                                    Some(false) => {
                                        // ui.label("not default");
                                    }
                                    None => {}
                                }
                            });
                        }
                    });
                }
            });
        }
    });
}

// actually the right way to do this is to have a seperate draw fn registered for single line ui
// UI is a late binding problem, we want runtime overrides of everything
// fn reflect_is_single_line(
//     reflect: &dyn Reflect,
//     type_registry: &TypeRegistry,
// ){
//     match reflect.reflect_ref(){
//         bevy::reflect::ReflectRef::Struct(_) => todo!(),
//         bevy::reflect::ReflectRef::TupleStruct(tuple_struct) => todo!(),
//         bevy::reflect::ReflectRef::Tuple(tuple) => todo!(),
//         bevy::reflect::ReflectRef::List(list) => todo!(),
//         bevy::reflect::ReflectRef::Array(array) => todo!(),
//         bevy::reflect::ReflectRef::Map(map) => todo!(),
//         bevy::reflect::ReflectRef::Set(set) => todo!(),
//         bevy::reflect::ReflectRef::Enum(_) => todo!(),
//         bevy::reflect::ReflectRef::Opaque(partial_reflect) => todo!(),
//     }
// }

fn reflect_is_default(reflect: &dyn Reflect, type_registry: &TypeRegistry) -> Option<bool> {
    if let Some(reflect_default) =
        type_registry.get_type_data::<ReflectDefault>(reflect.reflect_type_info().type_id())
    {
        reflect_default.default().reflect_partial_eq(reflect)
    } else {
        None
    }
}

fn show_duration(d: Duration) -> String{
    if d < Duration::from_millis(100){
        format!("{}ms", d.as_millis())
    }else if d < Duration::from_secs(60) {
        format!("{}s", d.as_secs())
    }else if d < Duration::from_secs(60*10) {
        format!("{:.1}m", d.as_secs_f32() / 60.0)
    }else if d < Duration::from_secs(60 * 60) {
        format!("{}m", d.as_secs() / 60)
    }else{
        let mut t = d.as_secs();
        let days = t / (3600*24);
        t -= days * (3600*24);
        let hours = t / 3600;
        t -= hours * (3600);
        let mins = t / 60;

        if days == 0 {
            format!("{hours}h{mins}m")
        }else{
            format!("{days}d{hours}h{mins}m")
        }
    }
}

// from bevy_inspector_egui
// pub fn show_docs() {
//     let mut end_idx = docs.len();
//     for (idx, ..) in docs.rmatch_indices("\n") {
//         let line = docs[idx + 1..].trim_start();
//         if line.starts_with("[") || line.is_empty() {
//             end_idx = idx;
//         } else {
//             break;
//         }
//     }

//     response.on_hover_ui(|ui| {
//         easymark(ui, &docs[..end_idx]);
//     });
// }

fn ui_for_entity_component(
    mut world: &mut World,
    // mut queue: Option<&mut CommandQueue>,
    component: CompInfo,
    entity: Entity,
    ui: &mut egui::Ui,
    type_registry: &TypeRegistry,
) {
    // todo side or top bar
    let name = component.short_name();
    let path = component.path_name();
    if !path.is_empty() {
        ui.label(egui::RichText::new(path).small().color(Color32::LIGHT_GRAY));
    }

    let missing = !world.entity(entity).contains_id(component.info.id());
    if missing {
        bevy_inspector::errors::component_does_not_exist(ui, entity, component.info.name());
    }

    // todo move me to show more stuff even when not registered
    let Some(type_info) = component.type_info else {
        if component.info.type_id().is_some() {
            bevy_inspector::errors::show_error(
                bevy_inspector_egui::restricted_world_view::Error::NoTypeRegistration(
                    component.type_info.type_id(),
                ),
                ui,
                &name,
            );
        } else {
            bevy_inspector::errors::no_type_id(ui, &name);
        }
        return;
    };

    if missing {
        return;
    }


    let ticks = TicksInfo::get(&world, entity, component.info.id());
    let added = ticks.added_time(world);
    let changed = ticks.changed_time(world);

    let id = ui.auto_id_with(component.info.id()).with(entity);

    // bevy_inspector_egui::egui_utils::show_docs(_response, type_docs);

    // create a context with access to the world except for the currently viewed componen
    let mut binding = RestrictedWorldView::from(&mut world);
    let (mut component_view, split_world) = binding.split_off_component((entity, type_info.type_id()));
    let mut cx = bevy_inspector_egui::reflect_inspector::Context {
        world: Some(split_world),
        #[allow(clippy::needless_option_as_deref)]
        // queue: queue.as_deref_mut(),
        queue: None, //TODO, this break AnimationContext buttons
        entity: Some(entity)
    };

    // EAS: this feels unnessecary
    // TODO: move to a function.
    let value = match component_view.get_entity_component_reflect(
        entity,
        type_info.type_id(),
        type_registry,
    ) {
        Ok(value) => value,
        Err(e) => {
            bevy_inspector::errors::show_error(e, ui, &name);
            return;
        }
    };

    // TODO show changes
    // if value.is_changed() {
    //     #[cfg(feature = "highlight_changes")]
    //     set_highlight_style(ui);
    // }

    let header = egui::CollapsingHeader::new(name).id_salt(id).default_open(true); //TODO move
    let response = header.show(ui, |ui| {
        ui.reset_style();

        let mut env = InspectorUi::for_bevy(type_registry, &mut cx);
        let id = id.with(component.info.id());
        let options = &();

        match value {
            bevy_inspector_egui::restricted_world_view::ReflectBorrow::Mutable(mut value) => {
                let changed = env.ui_for_reflect_with_options(
                    value.bypass_change_detection().as_partial_reflect_mut(),
                    ui,
                    id,
                    options,
                );

                if changed {
                    dbg!(entity);
                    value.set_changed();
                }
            }
            bevy_inspector_egui::restricted_world_view::ReflectBorrow::Immutable(value) => env
                .ui_for_reflect_readonly_with_options(value.as_partial_reflect(), ui, id, options),
        };

        if added.is_some() || changed.is_some() {
            ui.style_mut().interaction.selectable_labels = false;// required for interact(hover).on_hover_ui below to work
            ui.horizontal(|ui|{
                if let Some(d) = changed {
                    let text = format!("changed: {}  ", show_duration(d));
                    ui.label(RichText::new(text).small());
                }
                if let Some(d) = added {
                    let text = format!("added: {}  ", show_duration(d));
                    ui.label(RichText::new(text).small());
                }
            }).response.interact(Sense::hover()).on_hover_ui(|ui|{
                // TODO frames

                if let Some(by) =  ticks.changed_by {
                    // TODO function
                    if ui.link(RichText::new(by.to_string())).clicked(){
                        let _ = open_location(by).inspect_err(|e| error!("{}", e));
                    }
                }
            });
        }

        // NOTE: too much visual clutter
        // if let Some(by) =  ticks.changed_by {
        //     ui.label(RichText::new(by.to_string()).small());
        // }
    
        CollapsingHeader::new(RichText::new("Component Info:").weak().small()).show(ui, |ui| {
            let name = component.info.name();
            ui.label(format!("Name: {}", name));

            let mutable = component.info.mutable();
            ui.label(format!("Mutable: {}", mutable));

            let storage = component.info.storage_type();
            ui.label(format!("Storage: {:?}", storage));

            let layout = component.info.layout();
            ui.label(format!("Layout: {:?}", layout));
            // ui.label(format!("Layout: align = {}, size = {}", layout.align(), layout.size()));
            
            // let clone_behavior = match component.info.clone_behavior(){
            //     bevy::ecs::component::ComponentCloneBehavior::Default => "default",
            //     bevy::ecs::component::ComponentCloneBehavior::Ignore => "ignore",
            //     bevy::ecs::component::ComponentCloneBehavior::Custom(_) => "custom",
            // };
            let clone_behavior = component.info.clone_behavior();
            ui.label(format!("Clone Behavior: {:?}", clone_behavior));
            
            // TODO, bevy should make this introspectible
            let hooks = component.info.hooks();
            ui.label(format!("{:#?}", hooks));

            let is_send_and_sync = component.info.is_send_and_sync();
            ui.label(format!("Send+Sync: {}", is_send_and_sync));

            // TODO move whole section so I have a normal World
            // Or come up with better logic for spliting world access
            let world = env.context.world.as_ref().unwrap();
            ui.add_space(10.0);
            if component.info.required_components().iter_ids().next().is_some() {
                CollapsingHeader::new("Required Components").show(ui, |ui|{
                    for r in component.info.required_components().iter_ids(){
                        // TODO recursive, + handle cycles gracefully
                        let text = world.components().get_info(r).unwrap().name();
                        ui.label(text);
                    }
                });
            }else{
                ui.label("Required Components: None");
            }
            
            if !component.required_by.is_empty(){
                CollapsingHeader::new("Required By").show(ui, |ui|{
                    ui.label("**current entity's components only, FIXME**");
                    for r in component.required_by.iter(){
                        // TODO recursive, + handle cycles gracefully
                        let text = world.components().get_info(*r).unwrap().name();
                        ui.label(text);
                    }
                });
            }else{
                ui.label("Required By: None **for current entity, FIXME!**");
            }

            ui.add_space(10.0);
            if let Some(t) = ticks.ticks {
                // TODO this should be a struct that get's debug printed
                ui.label(format!("Ticks: {:?}", t));
                ui.label(format!("Added: {:?}", added));
                ui.label(format!("Changed: {:?}", changed));
            }
            if let Some(loc) = ticks.changed_by {
                if ui.link(format!("Changed By: {:?}", loc.to_string())).clicked(){
                    let _ = open_location(loc).inspect_err(|e| error!("{}", e));
                }
            }

            if let Some(type_info) = component.type_info {
                // let generics = type_info.generics();
                // ui.label(format!("{:?}", generics));

                // let file_lineno = "unimplemented";
                // ui.label(format!("File/LineNo: {}", file_lineno));

                if let Some(reg) = type_registry.get(type_info.type_id()){
                    ui.add_space(10.0);
                    ui.label("Type Data:");
                    
                    if let Some(def) = reg.data::<ReflectDefault>() {
                        CollapsingHeader::new("ReflectDefault").show(ui, |ui|{
                            env.ui_for_reflect_readonly(def.default().as_ref(), ui);
                        });
                    }

                    // type_info doesn't have type data
                    for (id,_) in reg.iter() {
                        // skip those manually handled above
                        if id == TypeId::of::<ReflectDefault>() {
                            continue;
                        }

                        match type_registry.get_type_info(id){
                            Some(v) => {
                                ui.label(v.type_path_table().short_path());
                            },
                            None => {
                                ui.label(format!("unknown type data {:?}", id));
                            },
                        }
                    }
                }

                if let Some(docs) = type_info.docs() {
                    CollapsingHeader::new("docs").show(ui, |ui| {
                        bevy_inspector_egui::egui_utils::easymark(ui, &docs);
                    });
                }
            } 
        });
    });

    // if let Some(queue) = queue.as_mut() {
    //     response.header_response.context_menu(|ui| {
    //         if ui.button("remove").clicked() {
    //             queue.push(move |world: &mut World| { world.entity_mut(entity).remove_by_id(component_id); })
    //         }
    //     });
    // }

    ui.reset_style();
}

pub struct CompInfo {
    pub info: ComponentInfo,
    pub type_info: Option<&'static TypeInfo>,
    pub registered: bool,
    pub required_by: SmallVec<[ComponentId; 4]>,
}

impl CompInfo {
    fn try_from_world(
        world: &World,
        c_id: ComponentId,
        registry: &TypeRegistry,
        entity: Option<Entity>,
    ) -> Option<Self> {
        let info = world.components().get_info(c_id)?.clone();
        let type_info = info.type_id().and_then(|id| registry.get_type_info(id));
        let registered = type_info.is_some();

        // Actually I think this is incorrect, get_reflect needs registration to work
        // if let Some(entity) = entity {
        //     // we can get the actual reflect type, and not have to rely on registry.
        //     world.get_reflect(entity, c_id).unwrap();
        // }

        Some(Self {
            info,
            type_info,
            required_by: Default::default(),
            registered,
        })
    }

    pub fn short_name(&self) -> String {
        let pattern = r"(?<name>\w+)(?:<(?<generic>.*)>)?$";
        static RE: OnceLock<regex::Regex> = OnceLock::new();
        let re = RE.get_or_init(|| regex::Regex::new(pattern).unwrap());

        if let Some(m) = re.captures(self.info.name()){
            let name = m.name("name").unwrap().as_str();
            let generic = m.name("generic");

            if generic.is_none(){
                name.into()
            }else{
                format!("{}<{}>", name, re.find(generic.unwrap().as_str()).unwrap().as_str())
            }
        }else{
            // weird
            panic!("weird");
            self.info.name().into()
        }
    }

    pub fn path_name(&self) -> String {
        let pattern = r"^(\w+::)+";
        static RE: OnceLock<regex::Regex> = OnceLock::new();
        let re = RE.get_or_init(|| regex::Regex::new(pattern).unwrap());
        if let Some(m) = re.find(self.info.name()){
            m.as_str()
        }else{
            ""
        }.into()
    }
}

// TODO generate at start of egui

type ComponentGroup = Vec<ComponentId>;

static BEVY_CRATES : OnceLock<HashSet<String>> = OnceLock::new();
fn is_bevy_crate(c: &str) -> bool {
    // BEVY_CRATES.get_or_init(|| vec![
    //     "bevy_ecs".to_string(),
    //     "bevy_transform".to_string(),
    //     "bevy_render".to_string(),
    //     "bevy_qizmo".to_string(),
    // ].into_iter().collect() ).contains(c)

    c.starts_with("bevy")
}

struct ComponentPlan {
    info: HashMap<ComponentId, CompInfo>,
    path: HashMap<String, HashSet<ComponentId>>, //TODO prefix trie
}

impl ComponentPlan {
    fn bundles_for_path(&self, path: &str) -> Option<Vec<ComponentGroup>> {
        let subset = self.path.get(path)?;
        let mut ret = HashMap::new();
        let mut seen = HashSet::new();
        for id in subset.iter() {
            if seen.contains(id) {
                // This prevents dups
                // XXX bundling logic has no clear correct solution in the case of multiheaded dep graphs
                // TODO bundle multiheaded (see bevy_mod_outline comps for use test)
                continue;
            }

            let info = self.info.get(id).unwrap();
            let mut req = Vec::new();
            let mut vals = HashSet::new();
            req.push(info.info.id());
            vals.insert(info.info.id());

            let mut n = 0;
            while let Some(id) = req.get(n).cloned() {
                n += 1;
                let info = self.info.get(&id).unwrap();
                req.extend(
                    info.info
                        .required_components()
                        .iter_ids()
                        .filter(|id| vals.insert(*id)),
                );

                // if already in ret, remove it. (should handle cycles gracefully)
                ret.remove(&id);
            }

            // sort in bundle
            req[1..].sort_by_key(|id| self.info.get(id).unwrap().short_name());

            // seen (XXX hack)
            for id in req.iter().cloned() {
                seen.insert(id);
            }

            // add to returns
            ret.insert(info.info.id(), req);
        }

        let mut ret = ret.into_iter().map(|a| a.1).collect::<Vec<_>>();
        ret.sort_by_key(|ids| self.info.get(&ids[0]).unwrap().short_name());

        Some(ret)
    }

    fn bundle_components(world: &mut World, entity: Entity, type_registry: &TypeRegistry) -> Self {
        // this is kinda annoying
        let mut required_groups: HashMap<ComponentId, CompInfo> = HashMap::new().into();
        let mut path_groups: HashMap<String, HashSet<ComponentId>> = HashMap::new().into();

        let binding = world.entity(entity);
        let arch = binding.archetype();
        for c_id in arch.components() {
            // This is an unacceptable short coming of the language
            let info = required_groups.entry(c_id).or_insert_with(|| {
                CompInfo::try_from_world(&world, c_id, type_registry, Some(entity)).unwrap()
            });
            let req: SmallVec<[ComponentId; 8]> =
                info.info.required_components().iter_ids().collect();

            let path = if let Some(type_info) = info.type_info {
                // todo override with registration.
                type_info.type_path().split("::").next().unwrap_or_default()
            } else {
                match info.info.name().split_once("::"){
                    Some(s) => s.0, // grab crate name from component name
                    None => "_unregistered_", // not a rust type
                }
            };
            path_groups.entry(path.into()).or_default().insert(c_id);

            for req in req {
                // NOTE: required components can be removed later
                let info = required_groups.entry(req).or_insert_with(|| {
                    CompInfo::try_from_world(&world, req, type_registry, Some(entity)).unwrap()
                });
                info.required_by.push(c_id);
            }
        }

        Self {
            info: required_groups,
            path: path_groups,
        }
    }


    fn paths(&self) -> Vec<String>{
        let mut keys : Vec<_> = self.path.keys().cloned().collect();
        keys.sort_by_cached_key(|id| (id == "", !is_bevy_crate(id), id.to_string()));
        keys
    }
}

fn relation_like(type_info: &'static TypeInfo) -> Option<String> {
    match type_info.kind() {
        bevy::reflect::ReflectKind::Struct => {
            for v in type_info.as_struct().unwrap().iter() {
                if v.is::<Entity>() {
                    return Some(format!(".{}", v.name()));
                }
            }
        }
        bevy::reflect::ReflectKind::TupleStruct => {
            for v in type_info.as_tuple_struct().unwrap().iter() {
                if v.is::<Entity>() {
                    return Some(format!(".{}", v.index()));
                }
            }
        }
        _ => {} // bevy::reflect::ReflectKind::Tuple => todo!(),
                // bevy::reflect::ReflectKind::List => todo!(),
                // bevy::reflect::ReflectKind::Array => todo!(),
                // bevy::reflect::ReflectKind::Map => todo!(),
                // bevy::reflect::ReflectKind::Set => todo!(),
                // bevy::reflect::ReflectKind::Enum => todo!(),
                // bevy::reflect::ReflectKind::Opaque => todo!(),
    };
    None
}
