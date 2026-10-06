//! Native UI component emitters.

use nexa_ir::Module;

use crate::generator::features::Features;

pub(crate) struct RenderScope<'a> {
    pub(crate) module: &'a Module,
    pub(crate) features: &'a Features,
}

pub(crate) mod accessibility;
pub(crate) mod appearance;
pub(crate) mod assets;
pub(crate) mod bottom_bar;
pub(crate) mod conditional;
pub(crate) mod content_unavailable;
pub(crate) mod controls;
pub(crate) mod custom_components;
pub(crate) mod dialogs;
pub(crate) mod direction;
pub(crate) mod images;
pub(crate) mod input;
pub(crate) mod keyboard;
pub(crate) mod layout;
pub(crate) mod lifecycle;
pub(crate) mod links;
pub(crate) mod lists;
pub(crate) mod navigation;
#[path = "components.rs"]
pub(crate) mod node_renderer;
pub(crate) mod refresh;
pub(crate) mod shared_elements;
pub(crate) mod sheets;
pub(crate) mod split_view;
pub(crate) mod status_bar;
pub(crate) mod system_icons;
pub(crate) mod text;

pub(super) use node_renderer::{render_children, render_node};
