#![allow(clippy::type_complexity)]
//! Default windows for the editor
// pub mod add;
pub mod assets;
pub mod cameras;
pub mod debug_settings;
pub mod diagnostics;
pub mod gizmos;
pub mod hierarchy;
pub mod inspector;
pub mod renderer;
pub mod resources;
// pub mod scenes;

pub mod prelude {
    pub use crate::assets::AssetsWindow;
    pub use crate::cameras::CameraWindow;
    pub use crate::debug_settings::DebugSettingsWindow;
    pub use crate::diagnostics::DiagnosticsWindow;
    pub use crate::gizmos::GizmosWindow;
    pub use crate::hierarchy::HierarchyWindow;
    pub use crate::inspector::InspectorWindow;
    pub use crate::renderer::RendererWindow;
    pub use crate::resources::ResourcesWindow;
}
