use super::*;

/// Layouts boxes stack-like.
///
/// The boxes are arranged along an axis, each layout gettings it's own "line".
#[derive(Debug, Clone)]
pub struct StackLayouter {
    ctx: StackContext,
    layouts: MultiLayout,
    /// Offset on secondary axis, anchor of the layout and the layout itself.
    boxes: Vec<(Size, Size2D, Layout)>,

    usable: Size2D,
    dimensions: Size2D,
    active_space: usize,
    include_empty: bool,
}

/// The context for stack layouting.
///
/// See [`LayoutContext`] for details about the fields.
#[derive(Debug, Clone)]
pub struct StackContext {
    pub spaces: LayoutSpaces,
    pub axes: LayoutAxes,
}

impl StackLayouter {
    /// Create a new stack layouter.
    pub fn new(ctx: StackContext) -> StackLayouter {
        let usable = ctx.spaces[0].usable().generalized(ctx.axes);
        StackLayouter {
            ctx,
            layouts: MultiLayout::new(),
            boxes: vec![],

            usable,
            active_space: 0,
            dimensions: start_dimensions(usable, ctx.axes),
            include_empty: true,
        }
    }

    /// This layouter's context.
    pub fn ctx(&self) -> StackContext {
        self.ctx
    }

    /// Add a sublayout.
    pub fn add(&mut self, layout: Layout) -> LayoutResult<()> {
        let size = layout.dimensions.generalized(self.ctx.axes);
        let mut new_dimensions = self.size_with(size);

        // Search for a suitable space to insert the box.
        while !self.usable.fits(new_dimensions) {
            if self.active_space == self.ctx.spaces.len() - 1 {
                return Err(LayoutError::NotEnoughSpace("box is to large for stack spaces"));
            }

            self.finish_layout()?;
            self.start_new_space(true);
            new_dimensions = self.size_with(size);
        }

        let ofset = self.dimensions.y;
        let anchor = self.ctx.axes.anchor(size);
        self.boxes.push((ofset, anchor, layout));

        self.dimensions.y += size.y;

        Ok(())
    }

    /// Add multiple sublayouts from a multi-layout.
    pub fn add_many(&mut self, layouts: MultiLayout) -> LayoutResult<()> {
        for layout in layouts {
            self.add(layout)?;
        }
        Ok(())
    }

    /// Add space after the last layout.
    pub fn add_space(&mut self, space: Size) -> LayoutResult<()> {
        if self.dimensions.y + space > self.usable.y {
            self.finish_layout()?;
            self.start_new_space(false);
        } else {
            self.dimensions.y += space;
        }

        Ok(())
    }

    /// Finish the layouting.
    ///
    /// The layouter is not consumed by this to prevent ownership problems.
    /// Nevertheless, it should not be used further.
    pub fn finish(&mut self) -> LayoutResult<MultiLayout> {
        if self.include_empty || !self.boxes.is_empty() {
            self.finish_layout()?;
        }
        Ok(std::mem::replace(&mut self.layouts, MultiLayout::new()))
    }

    /// Finish the current layout and start a new one in a new space.
    ///
    /// If `start_new_empty` is true, a new empty layout will be started. Otherwise,
    /// the new layout only appears once new content is added.
    pub fn finish_layout(&mut self) -> LayoutResult<()> {
        let mut actions = LayoutActionList::new();

        let space = self.ctx.spaces[self.active_space];
        let anchor = self.ctx.axes.anchor(self.usable);
        let factor = if self.ctx.axes.secondary.axis.is_positive() { 1 } else { -1 };
        let start = space.start();

        for (offset, layout_anchor, layout) in self.boxes.drain(..) {
            let general_position = anchor - layout_anchor + Size2D::with_y(offset * factor);
            let position = general_position.specialized(self.ctx.axes) + start;

            actions.add_layout(position, layout);
        }

        self.layouts.add(Layout {
            dimensions: if space.shrink_to_fit {
                self.dimensions.padded(space.padding)
            } else {
                space.dimensions
            },
            actions: actions.into_vec(),
            debug_render: true,
        });

        Ok(())
    }

    /// Set up layouting in the next space. Should be preceded by `finish_layout`.
    ///
    /// If `include_empty` is true, the new empty layout will always be added when
    /// finishing this stack. Otherwise, the new layout only appears if new
    /// content is added to it.
    pub fn start_new_space(&mut self, include_empty: bool) {
        self.active_space = (self.active_space + 1).min(self.ctx.spaces.len() - 1);
        self.usable = self.ctx.spaces[self.active_space].usable().generalized(self.ctx.axes);
        self.dimensions = start_dimensions(self.usable, self.ctx.axes);
        self.include_empty = include_empty;
    }

    /// The remaining space for new layouts.
    pub fn remaining(&self) -> Size2D {
        Size2D::new(self.usable.x, self.usable.y - self.dimensions.y)
            .specialized(self.ctx.axes)
    }

    /// The combined size of the so-far included boxes with the other size.
    fn size_with(&self, other: Size2D) -> Size2D {
        Size2D {
            x: crate::size::max(self.dimensions.x, other.x),
            y: self.dimensions.y + other.y,
        }
    }
}

fn start_dimensions(usable: Size2D, axes: LayoutAxes) -> Size2D {
    Size2D::with_x(match axes.primary.alignment {
        Alignment::Origin => Size::zero(),
        Alignment::Center | Alignment::End => usable.x,
    })
}