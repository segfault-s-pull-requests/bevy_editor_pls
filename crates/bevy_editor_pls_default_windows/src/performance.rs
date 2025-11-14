use std::time::Duration;

use bevy::{
    core_pipeline::{
        bloom::Bloom,
        prepass::{DepthPrepass, NormalPrepass},
    },
    diagnostic::{DiagnosticPath, DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    ecs::{query::QueryData, world::CommandQueue},
    pbr::{VolumetricFog, VolumetricLight},
    prelude::*,
    reflect::TypeRegistry,
};
use bevy_editor_pls_core::{
    editor_window::{EditorWindow, EditorWindowContext},
    AddEditorWindow,
};
use bevy_inspector_egui::{egui, reflect_inspector::ui_for_value};

#[derive(Debug, Clone, Default, Component)]
#[require(PerfWindowState)]
pub struct PerfWindow;

impl EditorWindow for PerfWindow {
    fn ui(&self, world: &mut World, cx: EditorWindowContext, ui: &mut egui::Ui) {
        quick_settings(ui, world);
        let diagnostics = match world.get_resource::<DiagnosticsStore>() {
            Some(diagnostics) => diagnostics,
            None => {
                ui.label("Diagnostics resource not available");
                return;
            }
        };
        let type_registry = world.resource::<AppTypeRegistry>().clone();
        let mut state = cx.get::<PerfWindowState>(world).unwrap().clone();
        perf_ui(ui, diagnostics, &mut state, &*type_registry.read());
        *cx.get_mut::<PerfWindowState>(world).unwrap() = state;
    }
}
impl Plugin for PerfWindow {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_editor_window::<Self>();
    }
}

fn quick_settings(ui: &mut egui::Ui, world: &mut World) {
    #[derive(QueryData)]
    struct Ident {
        entity: Entity,
        name: Option<&'static Name>,
    }

    fn name(id: IdentItem) -> String {
        id.name
            .map(|n| n.as_str().to_string())
            .unwrap_or(id.entity.to_string())
    }

    let mut queue = CommandQueue::default();
    ui.collapsing("camera", |ui| {
        for (id, mut camera, mut bloom, mut prepass, mut fog) in world
            .query::<(
                Ident,
                &mut Camera,
                Has<Bloom>,
                Has<DepthPrepass>,
                Has<VolumetricFog>,
            )>()
            .iter_mut(world)
        {
            if !camera.is_active || camera.target.as_image().is_some() {
                continue;
            }

            let e = id.entity;
            ui.label(name(id));
            ui.checkbox(&mut camera.hdr, "hdr");
            if ui.checkbox(&mut bloom, "bloom").changed() {
                queue.push(move |world: &mut World| {
                    if bloom {
                        world.entity_mut(e).insert(Bloom::default()); // TODO disable + restore
                    } else {
                        world.entity_mut(e).remove::<Bloom>(); // TODO disable + restore
                    }
                });
            }
            if ui.checkbox(&mut prepass, "prepass").changed() {
                queue.push(move |world: &mut World| {
                    if prepass {
                        world.entity_mut(e).insert(DepthPrepass::default()); // TODO disable + restore
                    } else {
                        world.entity_mut(e).remove::<DepthPrepass>(); // TODO disable + restore
                    }
                });
            }
            if ui.checkbox(&mut fog, "fog").changed() {
                queue.push(move |world: &mut World| {
                    if fog {
                        world.entity_mut(e).insert(VolumetricFog::default()); // TODO disable + restore
                    } else {
                        world.entity_mut(e).remove::<VolumetricFog>(); // TODO disable + restore
                    }
                });
            }
            ui.end_row();
        }
    });
    ui.collapsing("lights", |ui| {
        ui.heading("directional");
        ui.end_row();
        for (id, mut l, mut v, mut fog) in world
            .query::<(
                Ident,
                &mut DirectionalLight,
                &mut Visibility,
                Has<VolumetricLight>,
            )>()
            .iter_mut(world)
        {
            let e = id.entity;
            ui.label(name(id));
            ui.checkbox(&mut l.shadows_enabled, "shadow");
            let mut bool = *v.as_ref() != Visibility::Hidden;
            if ui.checkbox(&mut bool, "vis").changed() {
                v.toggle_inherited_hidden();
            }
            if ui.checkbox(&mut fog, "fog").changed() {
                queue.push(move |world: &mut World| {
                    if fog {
                        world.entity_mut(e).insert(VolumetricLight::default()); // TODO disable + restore
                    } else {
                        world.entity_mut(e).remove::<VolumetricLight>(); // TODO disable + restore
                    }
                });
            }
            ui.end_row();
        }
        ui.heading("spot");
        ui.end_row();
        for (id, mut l, mut v, mut fog) in world
            .query::<(Ident, &mut SpotLight, &mut Visibility, Has<VolumetricLight>)>()
            .iter_mut(world)
        {
            let e = id.entity;
            ui.label(name(id));
            ui.checkbox(&mut l.shadows_enabled, "shadow");
            let mut bool = *v.as_ref() != Visibility::Hidden;
            if ui.checkbox(&mut bool, "vis").changed() {
                v.toggle_inherited_hidden();
            }
            if ui.checkbox(&mut fog, "fog").changed() {
                queue.push(move |world: &mut World| {
                    if fog {
                        world.entity_mut(e).insert(VolumetricLight::default()); // TODO disable + restore
                    } else {
                        world.entity_mut(e).remove::<VolumetricLight>(); // TODO disable + restore
                    }
                });
            }
            ui.end_row();
        }
        ui.heading("point");
        ui.end_row();
        for (id, mut l, mut v, mut fog) in world
            .query::<(
                Ident,
                &mut PointLight,
                &mut Visibility,
                Has<VolumetricLight>,
            )>()
            .iter_mut(world)
        {
            let e = id.entity;
            ui.label(name(id));
            ui.checkbox(&mut l.shadows_enabled, "shadow");
            let mut bool = *v.as_ref() != Visibility::Hidden;
            if ui.checkbox(&mut bool, "vis").changed() {
                v.toggle_inherited_hidden();
            }
            if ui.checkbox(&mut fog, "fog").changed() {
                queue.push(move |world: &mut World| {
                    if fog {
                        world.entity_mut(e).insert(VolumetricLight::default()); // TODO disable + restore
                    } else {
                        world.entity_mut(e).remove::<VolumetricLight>(); // TODO disable + restore
                    }
                });
            }
            ui.end_row();
        }
    });
    queue.apply(world);
}

#[derive(Debug, Clone, Default, Reflect)]
pub enum DiagnosticMode {
    Average,
    #[default]
    Smoothed,
    Last,
}

#[derive(Debug, Clone, Default, Component, Reflect)]
#[reflect(Component)]
pub struct PerfWindowState {
    mode: DiagnosticMode,

    /// if false, represent free gpu time as an empty pie slice
    full: bool,
}

// TODO use average for pie divisions but adjust radius based on last frame, so that it's acurate area
fn perf_ui(
    ui: &mut egui::Ui,
    diagnostics: &DiagnosticsStore,
    state: &mut PerfWindowState,
    type_registry: &TypeRegistry,
) {
    ui_for_value(state.as_partial_reflect_mut(), ui, type_registry);
    ui.end_row();

    let mut total: f64 = 0.0;
    let frame_time = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .map(|diag| match state.mode {
            DiagnosticMode::Average => diag.average().unwrap(),
            DiagnosticMode::Smoothed => diag.smoothed().unwrap(),
            DiagnosticMode::Last => diag.measurement().unwrap().value,
        })
        .unwrap_or_default();

    let mut keys: Vec<_> = diagnostics
        .iter()
        .filter(|diag| {
            diag.path().as_str().ends_with("elapsed_gpu")
                && diag
                    .measurement()
                    .is_some_and(|m| m.time.elapsed() < Duration::from_millis(100))
        })
        .map(|diag| {
            let path: Vec<_> = diag.path().components().collect();
            let val = match state.mode {
                DiagnosticMode::Average => diag.average().unwrap(),
                DiagnosticMode::Smoothed => diag.smoothed().unwrap(),
                DiagnosticMode::Last => diag.measurement().unwrap().value,
            };

            total += val;

            (path, diag, val)
        })
        .collect();

    keys.sort_by_cached_key(|a| a.0.clone());

    let mut data: Vec<(f64, &str)> = keys
        .iter()
        .map(|(key, diag, val)| {
            let path = diag.path().as_str();
            let path = path.strip_prefix("render/").unwrap_or(path);
            let path = path.strip_suffix("/elapsed_gpu").unwrap_or(path);
            (*val, path)
        })
        .collect();

    if total < frame_time && !state.full {
        data.push((frame_time - total, "free"));
    }

    PieChart::new("render passes", data.as_slice()).show(ui);
}

// https://gist.github.com/rctlmk/d386fe0a9d6c36daa042192c970ed6e0
use std::f64::consts::TAU;

use egui_plot::{Legend, Plot, PlotPoint, PlotPoints, Polygon, Text};
use egui::{Align2, RichText};

const FULL_CIRCLE_VERTICES: f64 = 240.0;
const RADIUS: f64 = 1.0;

pub struct PieChart {
    name: String,
    sectors: Vec<Sector>,
}

impl PieChart {
    pub fn new<S: AsRef<str>, L: AsRef<str>>(name: S, data: &[(f64, L)]) -> Self {
        let sum: f64 = data.iter().map(|(f, _)| f).sum();

        let slices: Vec<_> = data.iter().map(|(f, n)| (f / sum, n)).collect();

        let step = TAU / FULL_CIRCLE_VERTICES;

        let mut offset = 0.0_f64;

        let sectors = slices
            .iter()
            .map(|(p, n)| {
                let vertices = (FULL_CIRCLE_VERTICES * p).round() as usize;

                let start = TAU * offset;
                let end = TAU * (offset + p);

                let sector = Sector::new(n, start, end, vertices, step);

                offset += p;

                sector
            })
            .collect();

        Self {
            name: name.as_ref().to_string(),
            sectors,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let sectors = self.sectors.clone();

        Plot::new(&self.name)
            .label_formatter(|_: &str, _: &PlotPoint| String::default())
            .show_background(false)
            .legend(Legend::default())
            .show_axes([false; 2])
            .show_grid(false)
            .allow_boxed_zoom(false)
            .allow_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .data_aspect(1.0)
            // .set_margin_fraction([0.7; 2].into()) // this won't prevent the plot from moving
            // `include_*` will lock it into place
            .include_x(-2.0)
            .include_x(2.0)
            .include_y(-2.0)
            .include_y(2.0)
            .show(ui, |plot_ui| {
                for sector in sectors.into_iter() {
                    let highlight = plot_ui
                        .pointer_coordinate()
                        .map(|p| sector.contains(&p))
                        .unwrap_or_default();

                    let Sector { name, points, .. } = sector;
                    if name == "free" {
                        continue;
                    }

                    plot_ui.polygon(
                        Polygon::new(PlotPoints::new(points))
                            .name(&name)
                            .highlight(highlight),
                    );

                    if highlight {
                        let p = plot_ui.pointer_coordinate().unwrap();

                        // TODO proper zoom
                        let text = RichText::new(&name).size(15.0).heading();
                        plot_ui.text(Text::new(p, text).name(&name).anchor(Align2::LEFT_BOTTOM));
                    }
                }
            });
    }
}

#[derive(Clone)]
struct Sector {
    name: String,
    start: f64,
    end: f64,
    points: Vec<[f64; 2]>,
}

impl Sector {
    pub fn new<S: AsRef<str>>(name: S, start: f64, end: f64, vertices: usize, step: f64) -> Self {
        let mut points = vec![];

        if end - TAU != start {
            points.push([0.0, 0.0]);
        }

        points.push([RADIUS * start.sin(), RADIUS * start.cos()]);

        for v in 1..vertices {
            let t = start + step * v as f64;
            points.push([RADIUS * t.sin(), RADIUS * t.cos()]);
        }

        points.push([RADIUS * end.sin(), RADIUS * end.cos()]);

        Self {
            name: name.as_ref().to_string(),
            start,
            end,
            points,
        }
    }

    pub fn contains(&self, &PlotPoint { x, y }: &PlotPoint) -> bool {
        let r = y.hypot(x);
        let mut theta = x.atan2(y);

        if theta < 0.0 {
            theta += TAU;
        }

        r < RADIUS && theta > self.start && theta < self.end
    }
}
