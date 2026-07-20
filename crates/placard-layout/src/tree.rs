use placard_style::ComputedStyle;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutNodeId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub enum BoxKind {
    Block,
    Text { content: String },

    InlineBackground,
}

/// `style` is `Rc`-shared rather than owned: many boxes (every word in a
/// run, every line-fragment of a wrapped `<span>`) commonly point at the
/// exact same computed style, and cloning the `Rc` is a refcount bump
/// instead of a full `ComputedStyle` copy.
#[derive(Debug, Clone)]
pub struct BoxNode {
    pub kind: BoxKind,
    pub rect: Rect,
    pub style: Rc<ComputedStyle>,
    pub children: Vec<LayoutNodeId>,
}

pub struct LayoutTree {
    boxes: Vec<BoxNode>,
    root: LayoutNodeId,
}

impl LayoutTree {
    pub(crate) fn empty() -> Self {
        Self {
            boxes: Vec::new(),
            root: LayoutNodeId(0),
        }
    }

    pub(crate) fn push(&mut self, node: BoxNode) -> LayoutNodeId {
        let id = LayoutNodeId(self.boxes.len() as u32);
        self.boxes.push(node);
        id
    }

    pub(crate) fn set_root(&mut self, id: LayoutNodeId) {
        self.root = id;
    }

    pub fn root(&self) -> LayoutNodeId {
        self.root
    }

    pub fn get(&self, id: LayoutNodeId) -> &BoxNode {
        &self.boxes[id.0 as usize]
    }

    pub fn children(&self, id: LayoutNodeId) -> &[LayoutNodeId] {
        &self.boxes[id.0 as usize].children
    }

    pub fn max_extent_y(&self) -> f32 {
        self.boxes
            .iter()
            .map(|b| b.rect.y + b.rect.height)
            .fold(0.0, f32::max)
    }

    /// The true rightmost extent of the document's *visible* content, as
    /// opposed to whatever containing width the tree happened to be built
    /// against. A block-level container fills its containing width whether
    /// or not its content actually reaches that far (no `flex-grow`/
    /// stretch means content commonly falls short of it) -- so a plain
    /// max over every box's edge, the way `max_extent_y` works, would just
    /// report back the width it was built against. This only counts boxes
    /// that paint something (text, an inline background, or a block with a
    /// background/border) or were given an explicit width by the author,
    /// skipping the invisible auto-sized wrapper `div`s that merely fill
    /// available space.
    pub fn max_extent_x(&self) -> f32 {
        self.boxes
            .iter()
            .filter(|b| Self::contributes_to_visible_extent(b))
            .map(|b| b.rect.x + b.rect.width)
            .fold(0.0, f32::max)
    }

    fn contributes_to_visible_extent(b: &BoxNode) -> bool {
        match &b.kind {
            BoxKind::Text { .. } | BoxKind::InlineBackground => true,
            BoxKind::Block => {
                b.style.background_color.a > 0
                    || b.style.border_width.iter().any(|&w| w > 0.0)
                    || b.style.width.is_some()
            }
        }
    }

    /// Shifts a box and every one of its descendants by `(dx, dy)`. Used to
    /// reposition an out-of-flow box after the fact, once its own size is
    /// known (e.g. a `bottom`/`right`-anchored absolutely positioned box
    /// that was first laid out at a placeholder origin).
    pub(crate) fn translate_subtree(&mut self, id: LayoutNodeId, dx: f32, dy: f32) {
        let mut stack = vec![id];
        while let Some(current) = stack.pop() {
            let node = &mut self.boxes[current.0 as usize];
            node.rect.x += dx;
            node.rect.y += dy;
            stack.extend(node.children.iter().copied());
        }
    }

    /// Overrides a box's own height (its children are left exactly where
    /// they were laid out). Used for `align-items: stretch` in a
    /// row-direction flex line: this engine has no notion of a "target
    /// height" to lay a box out against, so a stretched item's background/
    /// border grows to the cross size of its line without its content
    /// being recursively re-flowed or re-centered within it.
    pub(crate) fn set_height(&mut self, id: LayoutNodeId, height: f32) {
        self.boxes[id.0 as usize].rect.height = height;
    }
}
