pub mod camera_2d_panzoom;
pub mod camera_3d_free;
pub mod camera_3d_panorbit;
// use crate::scenes::NotInScene;

use std::any::type_name;
use std::marker::PhantomData;

use bevy::ecs::system::SystemState;
use bevy::render::camera::RenderTarget;
use bevy::render::view::RenderLayers;
use bevy::utils::HashSet;
use bevy::window::{PrimaryWindow, WindowRef};
use bevy::{prelude::*, render::primitives::Aabb};
use bevy_editor_pls_core::editor::EditorTabs;
use bevy_editor_pls_core::egui_dock::{self, LeafHighlighting};
use bevy_editor_pls_core::{set_if_neq, AddEditorWindow};
use bevy_editor_pls_core::{
    editor_window::{EditorWindow, EditorWindowContext},
    Editor,
    EditorEvent,
};
use bevy_inspector_egui::egui;
use camera_2d_panzoom::PanCamControls;
use camera_3d_free::FlycamControls;
use transform_gizmo_bevy::GizmoCamera;
// use bevy_mod_picking::prelude::PickRaycastSource;

use self::camera_3d_panorbit::PanOrbitCamera;

pub const EDITOR_RENDER_LAYER: usize = 19;

// Present on all editor cameras
#[derive(Component)]
pub struct EditorCamera;

// // Present only one the one currently active camera
// #[derive(Component)]
// pub struct ActiveEditorCamera;
#[derive(Default, Clone, Component, Debug)]
pub struct CameraWindow {
    camera: Option<Entity>,
}

fn target_window(camera: &Camera, primary: Entity) -> Option<Entity> {
    match camera.target {
        RenderTarget::Window(WindowRef::Primary) => Some(primary),
        RenderTarget::Window(WindowRef::Entity(e)) => Some(e),
        _ => None,
    }
}

#[derive(Default, Clone, Component, Debug)]
pub struct CameraWindowState {
    // make sure to keep the `ActiveEditorCamera` marker component in sync with this field
    // editor_cam: EditorCamKind,
    // pub show_ui: bool,
}

impl EditorWindow for CameraWindow {
    fn ui(&self, world: &mut World, cx: EditorWindowContext, ui: &mut egui::Ui) {
        // NOTE: moved from bevy_editor_pls_core

        // TODO controls are tied to camera not editor window, does that make sense?
        let mut state: SystemState<(
            Query<(
                Entity,
                &Camera,
                Option<&Name>,
                Has<EditorCamera>,
                (
                    Option<&PanOrbitCamera>,
                    Option<&FlycamControls>,
                    Option<&PanCamControls>,
                ),
            )>,
            Query<&CameraWindow>,
            Commands,
        )> = SystemState::new(world);

        let (cameras, windows, mut commands) = state.get_mut(world);
        let window = windows.get(cx.entity).expect("should be impossible");
        let camera_entity = window.camera.unwrap_or(cx.entity);
        let Ok((_, _, name, is_editor_cam, controls)) = cameras.get(camera_entity) else {
            warn!(
                "missing camera {:?} for window {}",
                window.camera, cx.entity
            );
            return;
        };

        if !is_editor_cam {
            return;
        }

        let mut camera_control_type = "disabled";
        if controls.0.is_some() {
            camera_control_type = PanOrbitCamera::NAME;
        }
        if controls.1.is_some() {
            camera_control_type = FlycamControls::NAME;
        }
        if controls.2.is_some() {
            camera_control_type = PanCamControls::NAME;
        }

        ui.horizontal(|ui| {
            if window.camera.is_some() {
                let namer = |name: Option<&Name>, entity: Entity| {
                    format!(
                        "{} {}",
                        name.map(|n| n.as_str()).unwrap_or_default(),
                        entity
                    )
                };
                let name = namer(name, camera_entity);
                ui.menu_button(name, |ui| {
                    for camera in cameras.iter() {
                        if ui.button(namer(camera.2, camera.0)).clicked() {
                            // TODO I have no idea what this might break in the editor toggle / camera viewport / is_active logic
                            let mut new = window.clone();
                            new.camera = Some(camera.0);
                            commands.entity(cx.entity).insert(new);
                        }
                    }
                });
            }

            ui.style_mut().spacing.button_padding = egui::vec2(2.0, 0.0);
            let height = ui.spacing().interact_size.y;
            ui.set_min_size(egui::vec2(ui.available_width(), height));

            // menu to select controls
            ui.menu_button(camera_control_type, |ui| {
                if ui.button(PanOrbitCamera::NAME).clicked() && controls.2.is_none() {
                    commands
                        .entity(camera_entity)
                        .reenable::<PanOrbitCamera>(Some(default()))
                        .disable::<PanCamControls>(true)
                        .disable::<FlycamControls>(true);
                    ui.close_menu();
                }
                if ui.button(FlycamControls::NAME).clicked() && controls.2.is_none() {
                    commands
                        .entity(camera_entity)
                        .reenable::<FlycamControls>(Some(default()))
                        .disable::<PanCamControls>(true)
                        .disable::<PanOrbitCamera>(true);
                    ui.close_menu();
                }
                if ui.button(PanCamControls::NAME).clicked() && controls.2.is_none() {
                    commands
                        .entity(camera_entity)
                        .reenable::<PanCamControls>(Some(default()))
                        .disable::<FlycamControls>(true)
                        .disable::<PanOrbitCamera>(true);
                    ui.close_menu();
                }
                if ui.button("disable").clicked() && controls.2.is_none() {
                    commands
                        .entity(camera_entity)
                        .disable::<PanCamControls>(true)
                        .disable::<FlycamControls>(true)
                        .disable::<PanOrbitCamera>(true);
                    ui.close_menu();
                }
            });
            // ui.checkbox(&mut state.show_ui, "UI"); //TODO?
        });
    }

    fn clear_background(&self) -> bool {
        false
    }
}

impl Plugin for CameraWindow {
    fn build(&self, app: &mut App) {
        // app.init_resource::<PreviouslyActiveCameras>();
        app.add_editor_window::<CameraWindow>();

        app.add_plugins(camera_2d_panzoom::PanCamPlugin)
            .add_plugins(camera_3d_free::FlycamPlugin)
            .add_plugins(camera_3d_panorbit::PanOrbitCameraPlugin)
            .add_systems(
                Update,
                set_editor_cam_active
                    .before(camera_3d_panorbit::CameraSystem::EditorCam3dPanOrbit)
                    .before(camera_3d_free::CameraSystem::EditorCam3dFree)
                    .before(camera_2d_panzoom::CameraSystem::EditorCam2dPanZoom),
            );

        // .add_systems(PreUpdate, focus_selected);
        // .add_systems(Update, initial_camera_setup);
        // app.add_systems(PreStartup, spawn_editor_camera);

        app.register_type::<Disabled<Camera>>();
        app.add_systems(
            PostUpdate,
            (
                toggle_editor_cam, // must run immediately before
                set_camera_viewports_and_enabled,
            )
                .chain()
                .after(bevy_editor_pls_core::EditorSet::UI)
                .before(bevy::render::camera::CameraUpdateSystem),
        );
    }
}

// fn set_active_editor_camera_marker(world: &mut World, editor_cam: EditorCamKind) {
//     let mut previously_active = world.query_filtered::<Entity, With<ActiveEditorCamera>>();
//     let mut previously_active_iter = previously_active.iter(world);
//     let previously_active = previously_active_iter.next();

//     assert!(
//         previously_active_iter.next().is_none(),
//         "there should be only one `ActiveEditorCamera`"
//     );

//     if let Some(previously_active) = previously_active {
//         world
//             .entity_mut(previously_active)
//             .remove::<ActiveEditorCamera>();
//     }

//     let entity = match editor_cam {
//         EditorCamKind::D2PanZoom => {
//             let mut state = world.query_filtered::<Entity, With<EditorCamera2dPanZoom>>();
//             state.iter(world).next().unwrap()
//         }
//         EditorCamKind::D3Free => {
//             let mut state = world.query_filtered::<Entity, With<EditorCamera3dFree>>();
//             state.iter(world).next().unwrap()
//         }
//         EditorCamKind::D3PanOrbit => {
//             let mut state = world.query_filtered::<Entity, With<EditorCamera3dPanOrbit>>();
//             state.iter(world).next().unwrap()
//         }
//     };
//     world.entity_mut(entity).insert(ActiveEditorCamera);
// }

fn set_editor_cam_active(
    editor: Res<Editor>,
    mut tabs: ResMut<EditorTabs>,
    camera_tabs: Query<(Entity, &CameraWindow)>,
    mut editor_cameras: Query<(
        Entity,
        &mut Camera,
        (
            Option<&mut camera_3d_free::FlycamControls>,
            Option<&mut camera_3d_panorbit::PanOrbitCamera>,
            Option<&mut camera_2d_panzoom::PanCamControls>,
        ),
    )>,
) {
    let focused = tabs
        .state
        .find_active_focused()
        .and_then(|(_, tab)| camera_tabs.get(tab.entity).ok())
        .map(|(e, w)| w.camera.unwrap_or(e));

    for (camera_entity, camera, controls) in editor_cameras.iter_mut() {
        let mut active = editor.active();
        active &= focused == Some(camera_entity);

        //TODO reimplemnent whatever logic was being used
        //enabled = active && editor.viewport_interaction_active();

        if let Some(mut c) = controls.0 {
            c.enable_look = active;
            c.enable_movement = active;
        }
        if let Some(mut c) = controls.1 {
            c.enabled = active;
        }
        if let Some(mut c) = controls.2 {
            c.enabled = active;
        }
    }
}

#[derive(Debug, Clone, Reflect, Component)]
#[reflect(Component)]
// on second thought this *is* an archetype change, not that it matters much either way
// #[component(storage = "SparseSet")]
struct Disabled<T>(T);

trait DisableCommandExt {
    fn disable<T>(&mut self, replace_existing: bool) -> &mut Self
    where
        T: Component;
    fn reenable<T>(&mut self, default_fn: Option<T>) -> &mut Self
    where
        T: Component;
}

impl<'a> DisableCommandExt for EntityCommands<'a> {
    fn disable<T: Component>(&mut self, replace_existing: bool) -> &mut Self {
        self.queue(move |mut entity: EntityWorldMut<'_>| {
            let Some(current) = entity.take::<T>() else {
                return;
            };
            if !replace_existing {
                if entity.get::<Disabled<T>>().is_some() {
                    return;
                }
            }
            entity.insert(Disabled(current));
        });
        self
    }

    fn reenable<T: Component>(&mut self, default: Option<T>) -> &mut Self {
        self.queue(move |mut entity: EntityWorldMut<'_>| {
            if let Some(Disabled(current)) = entity.take::<Disabled<T>>() {
                entity.insert(current);
            } else if let Some(default) = default {
                entity.insert(default);
            };
        });
        self
    }
}

/// ON: editor active
/// for all cameras targeting the editor window, without EditorCamera
/// - save their Camera component
/// - the rest is handled by set_camera_viewports_and_enabled
///
/// ON: editor deactivated
/// for all cameras with backup, restore
/// for all EditorCameras, disable
fn toggle_editor_cam(
    editor: Res<Editor>,
    mut editor_events: EventReader<EditorEvent>,
    // mut prev_active_cams: ResMut<PreviouslyActiveCameras>,
    mut cam_query: Query<(
        Entity,
        &mut Camera,
        Option<&Disabled<Camera>>,
        Has<EditorCamera>,
    )>,
    primary_window: Single<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    if editor_events.is_empty() {
        return;
    }

    editor_events.clear();

    if editor.active() {
        for (entity, camera, saved, is_editor_cam) in cam_query.iter_mut() {
            if target_window(&camera, *primary_window) == Some(editor.window())
                && !is_editor_cam
                && saved.is_none()
            {
                commands.entity(entity).insert(Disabled(camera.clone()));
            }
        }
    } else {
        for (entity, mut camera, saved, is_editor_cam) in cam_query.iter_mut() {
            if let Some(saved) = saved {
                *camera = saved.0.clone();
                commands.entity(entity).remove::<Disabled<Camera>>();
            }
            if is_editor_cam {
                camera.is_active = false;
            }
        }
    }
}

/// set all camera viewports to what they should be and disables cameras that aren't being used
/// This system runs after editor_ui and before camera update + render
/// only run when editor is active, only touches cameras sharing the editors window.
fn set_camera_viewports_and_enabled(
    editor: Res<Editor>,
    tabs: Res<EditorTabs>,
    root_window: Query<(
        &bevy_inspector_egui::bevy_egui::EguiContextSettings,
        &Window,
    )>,
    camera_tabs: Query<(Entity, &CameraWindow)>,
    mut cameras: Query<(Entity, &mut Camera)>,
    primary_window: Single<Entity, With<PrimaryWindow>>,
) {
    let Ok((egui_settings, root_window)) = root_window.get(editor.window()) else {
        warn!("missing editor window {}", editor.window());
        return;
    };

    let mut active_cameras = Vec::new();
    for (window_entity, window) in camera_tabs.iter() {
        // get the camera for this CameraWindow
        let Ok((camera_entity, mut camera)) =
            cameras.get_mut(window.camera.unwrap_or(window_entity))
        else {
            warn!("missing camera {:?} for {}", window.camera, window_entity);
            continue;
        };

        // very ugly code that ensures camera is targeting the editor window.
        // mainly so we don't have to specify the window when creating the camera
        // ugly so as not to trigger change detection
        match target_window(&camera, *primary_window) {
            Some(e) => {
                if e != editor.window() {
                    camera.target = RenderTarget::Window(WindowRef::Entity(editor.window()));
                }
            }
            _ => {
                warn!(
                    "camera window {} has camera {} with non window target {:?}",
                    window_entity, camera_entity, camera.target
                );
            }
        }

        let tab = tabs
            .state
            .iter_all_tabs()
            .find(|t| t.1.entity == window_entity)
            .unwrap(); // I apologize
        let node = &tabs.state[tab.0 .0][tab.0 .1];
        let egui_dock::Node::Leaf {
            rect: _,
            viewport,
            tabs,
            active,
            scroll: _,
            collapsed,
        } = node
        else {
            unreachable!()
        };

        let tab_index = tabs.iter().position(|t| t.entity == window_entity).unwrap();
        let visible = active.0 == tab_index && !collapsed;

        if !visible {
            continue;
        }

        let scale_factor = root_window.scale_factor() * egui_settings.scale_factor;

        let viewport_pos = viewport.left_top().to_vec2() * scale_factor;
        let viewport_pos = UVec2 {
            x: viewport_pos.x as u32,
            y: viewport_pos.y as u32,
        };

        let viewport_size = viewport.size() * scale_factor;
        if !viewport_size.is_finite() {
            panic!("editor viewport size is infinite");
        }
        let viewport_size = UVec2::new(
            (viewport_size.x as u32).max(1),
            (viewport_size.y as u32).max(1),
        );

        if camera.viewport.is_none() {
            camera.viewport = Some(default());
        }
        let mut camera_viewport = camera
            .reborrow()
            .map_unchanged(|v| v.viewport.as_mut().unwrap());

        // camera_viewport
        //     .reborrow()
        //     .map_unchanged(|v| &mut v.physical_position)
        //     .set_if_neq(viewport_pos);
        // I prefer my macro
        set_if_neq!(camera_viewport.physical_size, viewport_size);
        set_if_neq!(camera_viewport.physical_position, viewport_pos);

        active_cameras.push(camera_entity);
    }

    // set camera enabled
    for (camera_entity, mut camera) in cameras.iter_mut() {
        // ignore cameras not targeting the window of the editor
        target_window(&camera, *primary_window) == Some(editor.window());

        let active = active_cameras.contains(&camera_entity);
        set_if_neq!(camera.is_active, active);
    }
}

pub fn default_editor_cam() -> impl Bundle {
    let render_layers = RenderLayers::default().with(EDITOR_RENDER_LAYER);
    let editor_cam_priority = 100;

    (
        Camera3d::default(),
        Camera {
            //  Prevent multiple cameras from having the same priority.
            order: editor_cam_priority,
            is_active: false,
            ..default()
        },
        Transform::from_xyz(0.0, 2.0, 5.0),
        PanOrbitCamera::default(),
        EditorCamera,
        Name::new("Editor Camera"),
        // NotInScene,
        GizmoCamera,
        render_layers.clone(),
    )
}

// fn focus_selected(
//     mut editor_events: EventReader<EditorEvent>,
//     mut active_cam: Query<
//         (
//             &mut Transform,
//             Option<&mut PanOrbitCamera>,
//             Option<&mut OrthographicProjection>,
//         ),
//         With<ActiveEditorCamera>,
//     >,
//     selected_query: Query<
//         (&GlobalTransform, Option<&Aabb>, Option<&Sprite>),
//         Without<ActiveEditorCamera>,
//     >,
//     editor: Res<Editor>,
//     window: Query<&Window>,
// ) {
//     let Ok(window) = window.get(editor.window()) else {
//         //Prevent accumulation of irrelevant events
//         editor_events.clear();
//         return;
//     };

//     for event in editor_events.read() {
//         match *event {
//             EditorEvent::FocusSelected => (),
//             _ => continue,
//         }

//         let hierarchy = todo!();
//         if hierarchy.selected.is_empty() {
//             info!("Coudldn't focus on selection because selection is empty");
//             return;
//         }

//         let (bounds_min, bounds_max) = hierarchy
//             .selected
//             .iter()
//             .filter_map(|selected_e| {
//                 selected_query
//                     .get(selected_e)
//                     .map(|(&tf, bounds, sprite)| {
//                         let default_value = (tf.translation(), tf.translation());
//                         let sprite_size = sprite
//                             .map(|s| s.custom_size.unwrap_or(Vec2::ONE))
//                             .map_or(default_value, |sprite_size| {
//                                 (
//                                     tf * Vec3::from((sprite_size * -0.5, 0.0)),
//                                     tf * Vec3::from((sprite_size * 0.5, 0.0)),
//                                 )
//                             });

//                         bounds.map_or(sprite_size, |bounds| {
//                             (tf * Vec3::from(bounds.min()), tf * Vec3::from(bounds.max()))
//                         })
//                     })
//                     .ok()
//             })
//             .fold(
//                 (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
//                 |(acc_min, acc_max), (min, max)| (acc_min.min(min), acc_max.max(max)),
//             );

//         const RADIUS_MULTIPLIER: f32 = 2.0;

//         let bounds_size = bounds_max - bounds_min;
//         let focus_loc = bounds_min + bounds_size * 0.5;
//         let radius = if bounds_size.max_element() > f32::EPSILON {
//             bounds_size.length() * RADIUS_MULTIPLIER
//         } else {
//             RADIUS_MULTIPLIER
//         };

//         let (mut camera_tf, pan_orbit_cam, ortho) = active_cam.single_mut();

//         if let Some(mut ortho) = ortho {
//             camera_tf.translation.x = focus_loc.x;
//             camera_tf.translation.y = focus_loc.y;

//             ortho.scale = radius / window.width().min(window.height()).max(1.0);
//         } else {
//             camera_tf.translation = focus_loc + camera_tf.rotation.mul_vec3(Vec3::Z) * radius;
//         }

//         if let Some(mut pan_orbit_cam) = pan_orbit_cam {
//             pan_orbit_cam.focus = focus_loc;
//             pan_orbit_cam.radius = radius;
//         }

//         let len = hierarchy.selected.len();
//         let noun = if len == 1 { "entity" } else { "entities" };
//         info!("Focused on {} {}", len, noun);
//     }
// }

// fn initial_camera_setup(
//     mut has_decided_initial_cam: Local<bool>,
//     mut was_positioned_3d: Local<bool>,
//     mut was_positioned_2d: Local<bool>,

//     mut commands: Commands,
//     mut editor: ResMut<Editor>,

//     mut cameras: ParamSet<(
//         // 2d pan/zoom
//         Query<(Entity, &mut Transform), With<EditorCamera2dPanZoom>>,
//         // 3d free
//         Query<
//             (Entity, &mut Transform, &mut camera_3d_free::FlycamControls),
//             With<EditorCamera3dFree>,
//         >,
//         // 3d pan/orbit
//         Query<
//             (
//                 Entity,
//                 &mut Transform,
//                 &mut camera_3d_panorbit::PanOrbitCamera,
//             ),
//             With<EditorCamera3dPanOrbit>,
//         >,
//         Query<&Transform, (With<Camera2d>, Without<EditorCamera>)>,
//         Query<&Transform, (With<Camera3d>, Without<EditorCamera>)>,
//     )>,
// ) {
//     let cam2d = cameras.p3().get_single().ok().cloned();
//     let cam3d = cameras.p4().get_single().ok().cloned();

//     if !*has_decided_initial_cam {
//         let camera_state = editor.window_state_mut::<CameraWindow>().unwrap();

//         match (cam2d.is_some(), cam3d.is_some()) {
//             (true, false) => {
//                 camera_state.editor_cam = EditorCamKind::D2PanZoom;
//                 commands
//                     .entity(cameras.p0().single().0)
//                     .insert(ActiveEditorCamera);
//                 *has_decided_initial_cam = true;
//             }
//             (false, true) => {
//                 camera_state.editor_cam = EditorCamKind::D3PanOrbit;
//                 commands
//                     .entity(cameras.p2().single().0)
//                     .insert(ActiveEditorCamera);
//                 *has_decided_initial_cam = true;
//             }
//             (true, true) => {
//                 camera_state.editor_cam = EditorCamKind::D3PanOrbit;
//                 commands
//                     .entity(cameras.p2().single().0)
//                     .insert(ActiveEditorCamera);
//                 *has_decided_initial_cam = true;
//             }
//             (false, false) => return,
//         }
//     }

//     if !*was_positioned_2d {
//         if let Some(cam2d_transform) = cam2d {
//             if !cam2d_transform.rotation.is_finite()
//                 || !cam2d_transform.translation.is_finite()
//                 || !cam2d_transform.scale.is_finite()
//             {
//                 return;
//             };

//             let mut query = cameras.p0();
//             let (_, mut cam_transform) = query.single_mut();
//             *cam_transform = cam2d_transform;

//             *was_positioned_2d = true;
//         }
//     }

//     if !*was_positioned_3d {
//         if let Some(cam3d_transform) = cam3d {
//             if !cam3d_transform.rotation.is_finite()
//                 || !cam3d_transform.translation.is_finite()
//                 || !cam3d_transform.scale.is_finite()
//             {
//                 return;
//             };

//             {
//                 let mut query = cameras.p1();
//                 let (_, mut cam_transform, mut cam) = query.single_mut();
//                 *cam_transform = cam3d_transform;
//                 let (yaw, pitch, _) = cam3d_transform.rotation.to_euler(EulerRot::YXZ);
//                 cam.yaw = yaw;
//                 cam.pitch = pitch;
//             }

//             {
//                 let mut query = cameras.p2();
//                 let (_, mut cam_transform, mut cam) = query.single_mut();
//                 cam.radius = cam3d_transform.translation.distance(cam.focus);
//                 *cam_transform = cam3d_transform;
//             }

//             *was_positioned_3d = true;
//         }
//     }
// }
