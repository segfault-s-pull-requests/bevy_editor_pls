use bevy::ecs::bundle::DynamicBundle;
use bevy::ecs::change_detection::MutUntyped;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::system::Resource;
use bevy::ecs::world::{Mut, Ref};
use bevy::prelude::*;
use bevy::ptr::{Aligned, OwningPtr};
use bevy::reflect::{Reflect};
use bevy_inspector_egui::egui;
use polonius_the_crab::{polonius, polonius_break, polonius_return};
use std::any::TypeId;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ptr::NonNull;

/// at the moment this is just for organization.
#[derive(Debug, Default, Clone, Copy, Component)]
pub struct EditorWindowsCollection;

#[derive(Debug, Default, Clone, Copy, Component)]
pub struct EditorWindowInstance;

/// An editor window type
#[bevy_trait_query::queryable]
pub trait EditorWindow: 'static + Send + Sync + dyn_clone::DynClone {
    fn name(&self, world: &mut World, cx: EditorWindowContext) -> String {
        std::any::type_name::<Self>().trim_end_matches("Window").trim_end_matches("::").split("::").last().expect("split should never be empty").to_string()
    }

    // TODO I don't like this. Menu stuff could be it's own trait
    fn menu_name(&self) -> String {
        std::any::type_name::<Self>().trim_end_matches("Window").trim_end_matches("::").split("::").last().expect("split should never be empty").to_string()
    }

    fn default_size(&self) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn ui(&self, world: &mut World, cx: EditorWindowContext, ui: &mut egui::Ui);

    /// Ui shown in the `Open Window` menu item. By default opens the window as a floating window.
    fn menu_ui(&self, world: &mut World, mut cx: EditorWindowContext, ui: &mut egui::Ui){
        let _ = world;

        if ui.button(self.menu_name()).clicked() {
            
            self.spawn(world);
        }
    }

    fn spawn(&self, world: &mut World){
        let Some(id) = world.components().get_id(TypeId::of::<Self>()) else {
            error!("EditorWindow {} doesn't implement Component.", std::any::type_name::<Self>());
            return;
        };
        let data = dyn_clone::clone_box(self);
        // safety: I just grabbed id from the world. and the data is a Self
        // XXX I may or may not know how OwningPtr works
        // https://discord.com/channels/691052431525675048/742569353878437978/1371946114554658817

        unsafe {
            let data : *mut Self = Box::into_raw(data);
            let data : *mut ManuallyDrop<Self> = data as *mut ManuallyDrop<Self>; // to only free the box, not the recursive contents.
            let ptr : OwningPtr<'_, Aligned> = OwningPtr::new(NonNull::new_unchecked(<*mut _>::cast(data)));
            world.spawn_empty().insert_by_id(id, ptr);
            drop(Box::from_raw(data)) // frees the box without dropping the contents (which insert_by_id  has moved into ECS)
        }
    }

    fn clear_background(&self) -> bool {
        true
    }
}

// impl Clone for Box<dyn EditorWindow>
dyn_clone::clone_trait_object!(EditorWindow);

pub struct EditorWindowContext<'a> {
    // pub(crate) window_states: &'a mut HashMap<TypeId, EditorWindowState>,
    pub entity: Entity,
    pub(crate) internal_state: &'a mut crate::editor::EditorTabs,
}
impl EditorWindowContext<'_> {
    pub fn get<'a, M: Component>(&self, world: &'a World) -> Option<Ref<'a, M>> {
        if let Some(c) = world.entity(self.entity).get_ref::<M>() {
            return Some(c);
        }
        if let Some(l) = world.entity(self.entity).get::<Link<M>>() {
            if let Some(e) = world.entity(l.0).get_ref::<M>() {
                return Some(e);
            }
        }
        if let Some(r) = world.get_resource_ref::<DefaultLink<M>>() {
            match r.as_ref() {
                DefaultLink::Data(_) => {
                    let ret = r.map(|v| {
                        // if doing very simple things continues to require solving a rubiks cube, the this language will rightfully be used by no-one within 10 years
                        let DefaultLink::Data(v) = v else { panic!() };
                        v
                    });
                    return Some(ret);
                }
                DefaultLink::Link(l) => {
                    if let Some(e) = world.entity(*l).get_ref::<M>() {
                        return Some(e);
                    }
                }
            }
        }
        None
    }

    pub fn get_mut<'a, M: Component>(&self, mut world: &'a mut World) -> Option<Mut<'a, M>> {
        // dealing with borrow checker false positive
        // see: https://docs.rs/polonius-the-crab/latest/polonius_the_crab/index.html
        polonius!(|world| -> Option<Mut<'polonius, M>> {
            let res = world.get_mut::<M>(self.entity);
            if let Some(c) = res {
                polonius_return!(Some(c));
            }
        });

        if let Some(l) = world.get::<Link<M>>(self.entity) {
            let e = l.0;
            polonius!(|world| -> Option<Mut<'polonius, M>> {
                let res = world.get_mut::<M>(e);
                if let Some(c) = res {
                    polonius_return!(Some(c));
                }
            });
        }

        let link = polonius!(|world| -> Option<Mut<'polonius, M>> {
            let r = world.get_resource_mut::<DefaultLink<M>>();
            if let Some(mut r) = r {
                match r.as_mut() {
                    DefaultLink::Data(_) => {
                        let ret = r.map_unchanged(|link| match link {
                            DefaultLink::Data(d) => d,
                            _ => panic!(),
                        });
                        polonius_return!(Some(ret));
                    }
                    DefaultLink::Link(l) => Some(*l),
                }
            } else {
                None
            }
        });

        if let Some(l) = link {
            polonius!(|world| -> Option<Mut<'polonius, M>> {
                let res = world.get_mut::<M>(l);
                if let Some(c) = res {
                    polonius_return!(Some(c));
                }
            });
        }
        None
    }

    // pub fn state_mut<W: EditorWindow>(&mut self) -> Option<&mut W::State> {
    //     self.window_states
    //         .get_mut(&TypeId::of::<W>())
    //         .and_then(|s| s.downcast_mut::<W::State>())
    // }
    // pub fn state<W: EditorWindow>(&self) -> Option<&W::State> {
    //     self.window_states
    //         .get(&TypeId::of::<W>())
    //         .and_then(|s| s.downcast_ref::<W::State>())
    // }

    // pub fn state_mut_many<const N: usize>(
    //     &mut self,
    //     ids: [&TypeId; N],
    // ) -> [&mut (dyn Any + Send + Sync + 'static); N] {
    //     self.window_states
    //         .get_many_mut(ids)
    //         .unwrap()
    //         .map(|val| &mut **val)
    // }
    // pub fn state_mut_triplet<W1: EditorWindow, W2: EditorWindow, W3: EditorWindow>(
    //     &mut self,
    // ) -> Option<(&mut W1::State, &mut W2::State, &mut W3::State)> {
    //     let [a, b, c] = self.window_states.get_many_mut([
    //         &TypeId::of::<W1>(),
    //         &TypeId::of::<W2>(),
    //         &TypeId::of::<W3>(),
    //     ])?;

    //     let a = a.downcast_mut::<W1::State>()?;
    //     let b = b.downcast_mut::<W2::State>()?;
    //     let c = c.downcast_mut::<W3::State>()?;
    //     Some((a, b, c))
    // }

    // pub fn state_mut_pair<W1: EditorWindow, W2: EditorWindow>(
    //     &mut self,
    // ) -> Option<(&mut W1::State, &mut W2::State)> {
    //     assert_ne!(TypeId::of::<W1>(), TypeId::of::<W2>());

    //     let [a, b] = self
    //         .window_states
    //         .get_many_mut([&TypeId::of::<W1>(), &TypeId::of::<W2>()])?;

    //     let a = a.downcast_mut::<W1::State>()?;
    //     let b = b.downcast_mut::<W2::State>()?;
    //     Some((a, b))
    // }

    // pub fn open_floating_window<W: ?Sized + EditorWindow>(&mut self) {
    //     open_floating_window::<W>(self.internal_state)
    // }
}

// pub fn open_floating_window<W: ?Sized + EditorWindow>(
//     editor_internal_state: &mut crate::editor::EditorInternalState,
// ) {
//     let floating_window_id = editor_internal_state.next_floating_window_id();
//     let window_id = std::any::TypeId::of::<W>();
//     editor_internal_state
//         .floating_windows
//         .push(crate::editor::FloatingWindow {
//             window: window_id,
//             id: floating_window_id,
//             initial_position: None,
//         });
// }

#[derive(Debug, Clone, Resource, Reflect)]
pub enum DefaultLink<M> {
    Data(M),
    Link(Entity),
}

impl<M: Default> Default for DefaultLink<M> {
    fn default() -> Self {
        DefaultLink::Data(M::default())
    }
}

#[derive(Debug, Copy, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct Link<M>(pub Entity, #[reflect(ignore)] pub PhantomData<M>);
