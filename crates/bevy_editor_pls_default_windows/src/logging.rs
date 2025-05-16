use std::{collections::HashMap, env, sync::Arc};

/// a logging view and config.
/// Eventually it should be possible to select a component of an entity and, within the ui,
/// understand everything about how that component, on that entity, is being controlled, and controling
use bevy::{
    log::tracing_subscriber::{
        self,
        filter::Targets,
        fmt::format,
        layer::{Filter, SubscriberExt},
        registry,
        reload::{self, Handle},
        EnvFilter,
        Layer,
        Registry,
    },
    prelude::*,
    utils::tracing::{
        level_filters::LevelFilter,
        span,
        subscriber::Interest,
        Metadata,
        Subscriber,
    },
};
use bevy_editor_pls_core::{
    editor_window::{EditorWindow, EditorWindowContext},
    AddEditorWindow,
};
use bevy_egui::egui::{
    self,
    CentralPanel,
    Checkbox,
    CollapsingHeader,
    ComboBox,
    Id,
    Label,
    ScrollArea,
    SelectableLabel,
    Sense,
    SidePanel,
    TextEdit,
    TextStyle,
    TextWrapMode,
    Ui,
    WidgetText,
};

/// A logging view and config.
/// Eventually it should be possible to select a component of an entity and, within the ui,
/// understand everything about how that component, on that entity, is being controlled, and controling

#[derive(Debug, Clone, Reflect, Default, Component)]
#[reflect(Component)]
pub struct LoggingWindow;

impl EditorWindow for LoggingWindow {
    fn ui(&self, world: &mut World, cx: EditorWindowContext, ui: &mut Ui) {
        trace!(window = EntityLog(cx.entity).as_value(), "test");

        let lineheight = default_line_height(ui);

        let sub = world.resource::<TracingDynamicSubscriber>();

        let drop_down_simple = |mut current: LevelFilter, ui: &mut Ui| {
            let mut selected = None;
            ui.menu_button(current.to_string(), |ui| {
                for level in [
                    LevelFilter::OFF,
                    LevelFilter::ERROR,
                    LevelFilter::WARN,
                    LevelFilter::INFO,
                    LevelFilter::DEBUG,
                    LevelFilter::TRACE,
                ] {
                    if ui
                        .selectable_value(&mut current, level, level.to_string())
                        .clicked()
                    {
                        selected = Some(current);
                        ui.close_menu();
                    };
                }
            });
            selected
        };

        let drop_down = move |filter: &RwLock<Targets>, ui: &mut Ui| {
            let mut current = filter.read().default_level().unwrap_or(LevelFilter::OFF);

            if let Some(level) = drop_down_simple(current, ui) {
                let mut old = filter.write();
                *old = old.clone().with_default(level);
                drop(old);
                tracing_core::callsite::rebuild_interest_cache();
            };
        };

        let logs = sub.callsights.read().clone(); // XXX clone
                                                  // TODO how can I iterator over a RwLock without retaining the lock in the loop
        let mut keys = logs.keys().cloned().collect::<Vec<_>>();
        keys.sort_by_cached_key(|k| full_path(logs[k]));

        fn full_path(a: &Metadata<'static>) -> Vec<&'static str> {
            let mut v: Vec<&str> = a.target().split("::").collect();
            v.push(a.name());
            v
        }

        let id = ui.auto_id_with("select");
        let mut selected = ui.data_mut(|d| d.get_temp::<Identifier>(id));

        CentralPanel::default().show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Filter:");
                drop_down(&sub.filter, ui);
                ui.label("Meta Filter:")
                    .on_hover_text("this determines whether we can count disabled events");
                drop_down(&sub.meta_filter, ui);
            });

            CollapsingHeader::new("Filters:")
                .default_open(true)
                .show(ui, |ui| {
                    let guard = sub.filter.read();
                    let mut filters: Vec<_> = guard
                        .iter()
                        .map(|(a, b)| {
                            (
                                a.split("::").map(|s| s.to_string()).collect::<Vec<_>>(),
                                Some(b),
                            )
                        })
                        .collect();
                    drop(guard);
                    filters.sort();

                    let mut changed = false;
                    for (target, ref mut level) in filters.iter_mut() {
                        ui.horizontal(|ui| {
                            ui.label(target.join("::"));
                            if let Some(new) = drop_down_simple(level.unwrap(), ui) {
                                *level = Some(new);
                                changed = true;
                            };
                            if ui.button("X").clicked() {
                                *level = None;
                                changed = true;
                            }
                        });
                    }
                    if changed {
                        let mut filter = Targets::new();
                        if let Some(d) = sub.filter.read().default_level() {
                            filter = filter.with_default(d);
                        }
                        filter = filter.with_targets(
                            filters
                                .iter()
                                .filter_map(|(k, v)| v.map(|v| (k.join("::"), v))),
                        );

                        let mut old = sub.filter.write();
                        *old = filter;
                        drop(old);
                        tracing_core::callsite::rebuild_interest_cache();
                    }
                });

            #[derive(Default, Clone)]
            struct LogFilter {
                hide_levels: [bool; 5], // Trace, Debug, Info, Warn, Error
                kind: Option<&'static str>,
                filter_count: bool,
                filter_enabled: Option<bool>,
                regex: String,
            }
            let mut table_filter =
                ui.data_mut(|d| d.get_temp_mut_or_default::<LogFilter>(id).clone());

            let levels = [
                Level::ERROR,
                Level::WARN,
                Level::INFO,
                Level::DEBUG,
                Level::TRACE,
            ];

            let table = TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .sense(Sense::click())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::remainder());
            let table = table.header(lineheight, |mut ui| {
                ui.col(|ui| {
                    ui.menu_button(
                        table_filter
                            .filter_enabled
                            .map(|a| a.to_string())
                            .unwrap_or(" ANY ".into()),
                        |ui| {
                            let mut clicked = ui
                                .selectable_value(&mut table_filter.filter_enabled, None, "")
                                .clicked();
                            clicked |= ui
                                .selectable_value(
                                    &mut table_filter.filter_enabled,
                                    Some(true),
                                    "true",
                                )
                                .clicked();
                            clicked |= ui
                                .selectable_value(
                                    &mut table_filter.filter_enabled,
                                    Some(false),
                                    "false",
                                )
                                .clicked();
                            if clicked {
                                ui.close_menu();
                            }
                        },
                    )
                    .response
                    .on_hover_text("filter by enabled");
                });

                ui.col(|ui| {
                    ui.menu_button("filter", |ui| {
                        let mut all_checked = !table_filter.hide_levels.iter().any(|a| *a);
                        let any_checked = !table_filter.hide_levels.iter().all(|a| *a);
                        let indeterminate = any_checked && !all_checked;
                        if ui
                            .add(
                                Checkbox::new(&mut all_checked, "All").indeterminate(indeterminate),
                            )
                            .changed()
                        {
                            for check in &mut table_filter.hide_levels {
                                *check = !all_checked;
                            }
                        }
                        for (i, checked) in table_filter.hide_levels.iter_mut().enumerate() {
                            *checked = !*checked;
                            ui.checkbox(checked, levels[i].as_str());
                            *checked = !*checked;
                        }
                        if ui.button("close").clicked() {
                            ui.close_menu();
                        }
                    });
                });
                ui.col(|ui| {
                    ComboBox::from_id_salt("kind_select")
                        .selected_text(table_filter.kind.unwrap_or("Any"))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut table_filter.kind, Some("Any"), "Any");
                            ui.selectable_value(&mut table_filter.kind, Some("Span"), "Span");
                            ui.selectable_value(&mut table_filter.kind, Some("Event"), "Event");
                        });
                });
                ui.col(|ui| {
                    ui.checkbox(&mut table_filter.filter_count, "Count");
                });
                ui.col(|ui| {
                    ui.add(TextEdit::singleline(&mut table_filter.regex).hint_text("Filter regex"));
                });
            });

            let keys = keys
                .iter()
                .filter(|k| {
                    let d = logs[k];
                    let extra = sub
                        .extra
                        .read()
                        .get(&d.callsite())
                        .cloned()
                        .unwrap_or_default();
                    if table_filter.filter_count && extra.count.unwrap_or_default() == 0 {
                        return false;
                    }
                    match table_filter.kind {
                        Some("Event") => {
                            if !d.is_event() {
                                return false;
                            }
                        }
                        Some("Event") => {
                            if !d.is_span() {
                                return false;
                            }
                        }
                        _ => {}
                    }
                    if let Some(f) = table_filter.filter_enabled {
                        let enabled = sub.filter.read().would_enable(d.target(), d.level());
                        if enabled != f {
                            return false;
                        }
                    }

                    for i in 0..5 {
                        if *d.level() == levels[i] && table_filter.hide_levels[i] {
                            return false;
                        }
                    }

                    if !table_filter.regex.is_empty() {
                        if let Ok(r) = Regex::new(&table_filter.regex) {
                            if !r.is_match(d.target()) {
                                return false;
                            }
                        }
                    }

                    true
                })
                .collect::<Vec<_>>();

            table.body(|ui| {
                ui.rows(lineheight, keys.len(), |mut ui| {
                    let data = logs[&keys[ui.index()]];
                    let enabled = sub.filter.read().would_enable(data.target(), data.level());
                    let extra = sub
                        .extra
                        .read()
                        .get(&data.callsite())
                        .cloned()
                        .unwrap_or_default();
                    ui.set_selected(selected == Some(data.callsite()));
                    assert_eq!(data.callsite(), *keys[ui.index()]);

                    ui.col(|ui| {
                        if !enabled {
                            ui.label("❌");
                        };
                    });
                    ui.col(|ui| {
                        ui.label(data.level().as_str());
                    });
                    ui.col(|ui| {
                        ui.label(kind(data));
                    });

                    ui.col(|ui| {
                        let s = extra.count.map(|n| n.to_string()).unwrap_or_default();
                        ui.label(s);
                    });
                    ui.col(|ui| {
                        ui.add(
                            Label::new(format!("{} {}", data.target(), data.name()))
                                .wrap_mode(TextWrapMode::Truncate)
                                .selectable(false), // or else table rows aren't clickable
                        );
                    });
                    ui.response().context_menu(|ui| {
                        if ui.button("disable").clicked() {
                            let mut new = sub.filter.read().clone();
                            new = new.with_target(data.target(), LevelFilter::OFF);

                            let mut old = sub.filter.write();
                            *old = new;
                            drop(old);
                            tracing_core::callsite::rebuild_interest_cache();
                            ui.close_menu();
                        }

                        ui.strong(format!("add filter for:"));
                        let mut parts: Vec<_> = data.target().split("::").collect();
                        parts.push(data.name());

                        for i in 1..parts.len() {
                            let s = parts.iter().cloned().take(i).collect::<Vec<_>>().join("::");
                            if ui.button(&s).clicked() {
                                let mut new = sub.filter.read().clone();
                                new = new.with_target(s, *data.level());

                                let mut old = sub.filter.write();
                                *old = new;
                                drop(old);
                                tracing_core::callsite::rebuild_interest_cache();
                            }
                        }
                    });
                    if ui.response().clicked() {
                        selected = Some(data.callsite());
                    }
                });
            });

            ui.data_mut(|d| d.insert_temp::<LogFilter>(id, table_filter.clone()));
        });

        if selected.is_some() {
            ui.data_mut(|d| d.insert_temp::<Identifier>(id, selected.clone().unwrap()));
        }

        if let Some(k) = selected {
            let _ = SidePanel::right("info")
                .resizable(true)
                .show_inside(ui, |ui| {
                    MetadataWidget(logs[&k]).ui(ui);
                    let extra = sub.extra.read().get(&k).cloned();
                    let registry = world.resource::<AppTypeRegistry>();

                    // TODO refactor this to no use
                    bevy_inspector_egui::reflect_inspector::ui_for_value_readonly(
                        extra.as_reflect(),
                        ui,
                        &registry.read(),
                    );
                })
                .response
                .interact(Sense::all()); // prevents input passing through to table underneath
        }
    }
}

impl Plugin for LoggingWindow {
    fn build(&self, app: &mut App) {
        app.add_editor_window::<Self>();
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EntityLog(Entity);

impl Valuable for EntityLog {
    fn as_value(&self) -> valuable::Value<'_> {
        valuable::Value::Structable(self)
    }

    fn visit(&self, visit: &mut dyn valuable::Visit) {
        visit.visit_unnamed_fields(&[self.0.to_bits().as_value()])
    }
}

impl Structable for EntityLog {
    fn definition(&self) -> StructDef<'_> {
        valuable::StructDef::new_dynamic("EntityLog", Fields::Unnamed(0))
    }
}

use bevy::utils::tracing;
use bevy_inspector_egui::dropdown::DropDownBox;
use egui_extras::{Column, TableBuilder};
use parking_lot::RwLock;
use regex::Regex;
use tracing_core::{callsite::Identifier, field::FieldSet, Level};
use valuable::{Fields, StructDef, Structable, Valuable};

use crate::metrics;

#[derive(Debug, Clone, Default, Reflect)]
pub struct ExtraCallsightData {
    pub count: Option<usize>,
    pub is_registered: bool,
    pub was_leaked: bool,
}

/// A tracing subscriber built for runtime reconfigurable log interest
#[derive(Clone, Resource, Default)]
pub struct TracingDynamicSubscriber {
    callsights: Arc<RwLock<HashMap<tracing::callsite::Identifier, &'static Metadata<'static>>>>,
    extra: Arc<RwLock<HashMap<tracing::callsite::Identifier, ExtraCallsightData>>>,
    filter: Arc<RwLock<Targets>>,
    meta_filter: Arc<RwLock<Targets>>,
}

/// Implement a ring buffer seperately for each callsite.
/// further seperate callsite based on system field for spans
///     make this parametric based on Field (which is string + callsite)
/// hash logs so that identical ones get collapsed
///
/// entity system and component querys to the db can be accelerated via archetype style lookup
/// use spacetimedb?
///
/// TODO: NEVER log every frame events to console.
pub struct RetainedLog {}

impl<S: Subscriber> Layer<S> for TracingDynamicSubscriber {
    fn register_callsite(
        &self,
        metadata: &'static bevy::utils::tracing::Metadata<'static>,
    ) -> bevy::utils::tracing::subscriber::Interest {
        self.callsights
            .write()
            .insert(metadata.callsite(), metadata);

        let count = Filter::<S>::callsite_enabled(&*self.meta_filter.read(), metadata).is_always();
        let record = Filter::<S>::callsite_enabled(&*self.filter.read(), metadata).is_always();

        let interest = match (count, record) {
            (false, false) => Interest::never(),
            _ => Interest::sometimes(), //TODO return always?
        };

        // initialize the count, with fully disabled (ie. uncountable) logs set to None
        let mut guard = self.extra.write();
        let entry = guard.entry(metadata.callsite()).or_default();
        entry.is_registered = true;
        match interest.is_never() {
            true => {
                entry.count = None;
            }
            false => {
                entry.count = Some(0);
            }
        };

        interest
    }

    fn enabled(
        &self,
        metadata: &Metadata<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        let mut leaked = false;
        if !self.callsights.read().contains_key(&metadata.callsite()) {
            dbg!(metadata.name()); //XXX tracing_log adapter results in register callsight never being called.

            // tracing crate should be changed to
            // 1. make FieldSet copy or at least clone.
            //
            // also, why does this method not get passed an &'static Metadata<'static> ?
            // TODO is there a better option than leaking?, store in the subscriber somehow?
            let fields: Vec<&'static str> = metadata.fields().iter().map(|v| v.name()).collect();
            let fields: &'static [&'static str] = Box::leak(fields.into_boxed_slice());

            let leak = |s: &str| -> &'static str { Box::leak(s.to_string().into_boxed_str()) };

            let meta = Metadata::<'static>::new(
                metadata.name(),
                leak(metadata.target()),
                *metadata.level(),
                metadata.file().map(leak),
                metadata.line(),
                metadata.module_path().map(leak),
                FieldSet::new(fields, metadata.callsite()),
                match (metadata.is_event(), metadata.is_span()) {
                    (true, false) => tracing_core::Kind::EVENT,
                    (false, true) => tracing_core::Kind::SPAN,
                    _ => panic!(),
                },
            );
            // leak: should be okay since this should only be called once per callsite. Could be an issue if program is dynamically creating new callsite id's though.
            let static_meta: &'static Metadata<'static> = Box::leak(Box::new(meta));
            self.callsights
                .write()
                .insert(static_meta.callsite(), static_meta);
            leaked = true;
        }
        {
            let mut guard = self.extra.write();
            let entry = guard.entry(metadata.callsite()).or_default();
            entry.count = Some(entry.count.unwrap_or_default() + 1);
            if leaked {
                entry.was_leaked = true;
            }
        }
        Filter::<S>::enabled(&*self.filter.read(), metadata, &ctx)
    }

    fn on_new_span(
        &self,
        attrs: &span::Attributes<'_>,
        id: &span::Id,
        ctx: bevy::log::tracing_subscriber::layer::Context<'_, S>,
    ) {
    }

    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: bevy::log::tracing_subscriber::layer::Context<'_, S>,
    ) {
        struct TestVisitor;
        impl tracing::field::Visit for TestVisitor {
            fn record_debug(&mut self, field: &tracing_core::Field, value: &dyn std::fmt::Debug) {}
            fn record_value(&mut self, field: &tracing_core::Field, value: valuable::Value<'_>) {
                value.visit(&mut TestVisitor);
            }
        }
        impl valuable::Visit for TestVisitor {
            fn visit_value(&mut self, value: valuable::Value<'_>) {
                if let Some(v) = value.as_structable() {
                    if v.definition().name() == "EntityLog" {
                        dbg!(value);
                    }
                }
            }
        }

        event.record(&mut TestVisitor)
    }
}

impl TracingDynamicSubscriber {
    pub fn new() -> Self {
        let filter = env::var("RUST_LOG")
            .unwrap_or("info".into())
            .parse::<Targets>()
            .unwrap();
        // let (filter, reload_handle) = reload::Layer::new(filter);

        let filter = env::var("RUST_LOG")
            .unwrap_or("info".into())
            .parse::<Targets>()
            .unwrap();
        let meta_filter = env::var("RUST_LOG_META")
            .unwrap_or("trace".into())
            .parse::<Targets>()
            .unwrap();
        let layer = TracingDynamicSubscriber {
            callsights: default(),
            extra: default(),
            filter: Arc::new(filter.into()),
            meta_filter: Arc::new(meta_filter.into()),
        };

        // tracing_subscriber::registry().with(filter).with(layer);
        layer
    }
}

struct MetadataWidget(&'static Metadata<'static>);

impl MetadataWidget {
    pub fn ui(self, ui: &mut Ui) {
        ui.label(format!("Name: {}", self.0.name()));
        ui.label(format!("Kind: {}", kind(self.0)));
        ui.label(format!("Target: {}", self.0.target()));
        ui.label(format!("Level: {}", self.0.level().as_str()));

        ui.label(format!(
            "Module Path: {}",
            self.0.module_path().unwrap_or("<none>")
        ));
        ui.label(format!("File: {}", self.0.file().unwrap_or("<none>")));
        ui.label(format!(
            "Line: {}",
            self.0.line().map_or("<none>".into(), |l| l.to_string())
        ));
        if ui.button("Open").clicked() {
            if let Err(e) = crate::utils::open::open_file_at_line(self.0) {
                eprintln!("Failed to open file: {}", e);
            }
        }

        ui.separator();
        ui.label("Fields:");
        for field in self.0.fields().iter() {
            ui.label(format!("- {}", field.name()));
        }
    }
}

fn kind(metadata: &Metadata<'_>) -> &'static str {
    match (metadata.is_event(), metadata.is_span()) {
        (true, false) => "event",
        (false, true) => "span",
        _ => "secret third thing",
    }
}

pub struct Selection<K> {
    id: Id,
    _phantom: std::marker::PhantomData<K>,
}

impl<K: Clone + PartialEq + 'static + Send + Sync> Selection<K> {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn ui<'a, I, F, T>(&self, ui: &mut Ui, values: I, show: F) -> Option<K>
    where
        I: IntoIterator<Item = K>,
        F: Fn(&K) -> T,
        T: Into<WidgetText>,
    {
        let mut selected = ui.data_mut(|d| d.get_temp::<K>(self.id));

        ScrollArea::vertical().show(ui, |ui| {
            for value in values {
                let label = show(&value);
                let selected_now = selected.as_ref() == Some(&value);

                if ui.selectable_label(selected_now, label).clicked() {
                    selected = Some(value.clone());
                    ui.data_mut(|d| d.insert_temp(self.id, value.clone()));
                }
            }
        });

        selected
    }
}

fn default_line_height(ui: &Ui) -> f32 {
    let text_style = TextStyle::Body;
    let font_id = ui.style().text_styles[&text_style].clone();

    ui.ctx().fonts(|fonts| fonts.row_height(&font_id))
}
