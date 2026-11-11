//! Typed, platform-neutral layout contracts shared by the compiler and native
//! backends. These plans deliberately keep dynamic window insets, accessory
//! extents, gesture regions, and fixed design spacing as separate sources.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LayoutEdge {
    Top,
    Bottom,
    Leading,
    Trailing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DynamicInsetCategory {
    /// Visual system chrome such as status and navigation bars.
    SystemBars,
    /// Hardware display cutouts and camera islands that reduce usable pixels.
    DisplayCutout,
    /// Curved-edge display regions that may clip content.
    Waterfall,
    /// System-owned touch regions reserved for back and other gestures.
    SystemGestures,
    /// The non-overridable subset of system gesture regions.
    MandatorySystemGestures,
    /// System-owned tappable controls such as the navigation affordance.
    TappableElement,
    /// The on-screen keyboard (IME), which is handled separately from visual safe areas.
    Ime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LayoutInsetSource {
    Dynamic(DynamicInsetCategory),
    EdgeAccessory,
    FixedDesignSpacing,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScrollAxis {
    #[default]
    Vertical,
    Horizontal,
    Both,
}

impl ScrollAxis {
    pub fn supports_edge(self, edge: LayoutEdge) -> bool {
        match edge {
            LayoutEdge::Top | LayoutEdge::Bottom => {
                matches!(self, Self::Vertical | Self::Both)
            }
            LayoutEdge::Leading | LayoutEdge::Trailing => {
                matches!(self, Self::Horizontal | Self::Both)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScrollOwnerKind {
    ScrollView,
    FastList,
    Form,
    KeyboardAware,
    PagePager,
}

/// Stable within one depth-first analysis of a typed UI subtree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScrollOwnerId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScrollOwnerDescriptor {
    pub id: ScrollOwnerId,
    pub kind: ScrollOwnerKind,
    pub axes: ScrollAxis,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScrollOwnerResolution {
    None,
    Unique(ScrollOwnerDescriptor),
    Ambiguous(Vec<ScrollOwnerDescriptor>),
    InvalidReference(ScrollOwnerId),
    /// An inset surrounds a navigation host. Its routes are alternatives that
    /// share the host's endpoint reservation, not concurrent scroll owners.
    RouteHost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScrollHeaderBehavior {
    Pinned,
    Collapse,
    HideOnScroll,
}

/// Keyboard relationship for a keyboard-aware content scope. `Stay` leaves
/// the scope anchored to the physical window; `Avoid` opts into moving or
/// resizing the scope above the software keyboard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyboardBehavior {
    #[default]
    Stay,
    Avoid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScrollOwnerSelection {
    Automatic,
    Explicit(ScrollOwnerId),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SafeAreaInsetPlan {
    pub edge: LayoutEdge,
    /// A unique inferred owner or route host receives endpoint accommodation.
    /// `None` and `Ambiguous` leave placement intact without guessing.
    pub scroll_owner: ScrollOwnerResolution,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScrollHeaderPlan {
    pub behavior: ScrollHeaderBehavior,
    /// Used by collapsing headers; measured in platform-independent logical units.
    pub collapsed_height: Option<f32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AdaptiveViewModifierPlan {
    SafeAreaInset(SafeAreaInsetPlan),
    ScrollHeader(ScrollHeaderPlan),
}

#[cfg(test)]
mod tests {
    use super::{
        DynamicInsetCategory, LayoutEdge, LayoutInsetSource, ScrollAxis, ScrollHeaderBehavior,
        ScrollOwnerId, ScrollOwnerKind, ScrollOwnerSelection,
    };

    #[test]
    fn edge_axis_matching_is_explicit() {
        assert!(ScrollAxis::Vertical.supports_edge(LayoutEdge::Top));
        assert!(!ScrollAxis::Vertical.supports_edge(LayoutEdge::Leading));
        assert!(ScrollAxis::Horizontal.supports_edge(LayoutEdge::Trailing));
        assert!(ScrollAxis::Both.supports_edge(LayoutEdge::Bottom));
    }

    #[test]
    fn inset_sources_keep_dynamic_and_design_values_distinct() {
        assert_eq!(
            LayoutInsetSource::Dynamic(DynamicInsetCategory::Ime),
            LayoutInsetSource::Dynamic(DynamicInsetCategory::Ime)
        );
        assert_ne!(
            LayoutInsetSource::Dynamic(DynamicInsetCategory::SystemBars),
            LayoutInsetSource::FixedDesignSpacing
        );
        let _selection = ScrollOwnerSelection::Explicit(ScrollOwnerId(2));
        let _owner = ScrollOwnerKind::Form;
        let _header = ScrollHeaderBehavior::Collapse;
    }
}
