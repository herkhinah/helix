use helix_view::{
    graphics::Margin,
    theme::{Modifier, Style},
};
use tui::widgets::{Block, Borders, Widget};

use crate::{
    compositor::{Component, Compositor, EventResult},
    key,
};

use helix_view::input::Event;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Index(pub usize);

impl std::ops::Deref for Index {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub trait TreeItem {
    type Data;

    fn child(&self, row: usize) -> Index;

    fn child_count(&self) -> usize;

    fn child_index(&self) -> usize;

    fn data(&self, column: usize) -> Self::Data;

    fn parent(&self) -> Option<Index>;

    fn index(&self) -> Index;

    fn render(&self) -> &str;
}

pub trait TreeModel {
    type Data: TreeItem;

    fn get_roots(&self) -> &[Index];

    fn get_first(&self) -> Option<Index> {
        let roots = self.get_roots();
        if roots.len() > 0 {
            return Some(roots[0]);
        }

        None
    }

    fn get_item(&self, ix: Index) -> &Self::Data;

    fn parent(&self, ix: &Index) -> Option<Index>;

    fn row_count(&self) -> usize;
    fn column_count(&self) -> usize;

    fn depth(&self, ix: Index) -> usize {
        let mut depth = 0;
        let mut iter = ix;

        while let Some(parent) = self.parent(&ix) {
            depth += 1;
            iter = parent;
        }

        depth
    }

    fn next_sibling(&self, ix: Index) -> Option<Index> {
        let item = self.get_item(ix);
        match item.parent() {
            Some(parent) => {
                let parent = self.get_item(parent);
                let child_index = item.child_index();
                if parent.child_count() > child_index + 1 {
                    return Some(parent.child(child_index + 1));
                }
                None
            }
            None => {
                let child_index = item.child_index();
                let roots = self.get_roots();
                if roots.len() > child_index + 1 {
                    return Some(roots[child_index + 1]);
                }
                None
            }
        }
    }

    fn prev_sibling(&self, ix: Index) -> Option<Index> {
        let item = self.get_item(ix);
        match item.parent() {
            Some(parent) => {
                let parent = self.get_item(parent);
                let child_index = item.child_index();
                if child_index > 0 {
                    return Some(parent.child(child_index - 1));
                }
                None
            }
            None => {
                let child_index = item.child_index();
                let roots = self.get_roots();
                if child_index > 0 {
                    return Some(roots[child_index - 1]);
                }
                None
            }
        }
    }

    fn next_uncle(&self, ix: Index) -> Option<Index> {
        let item = self.get_item(ix);
        match item.parent() {
            Some(parent) => {
                let parent = self.get_item(parent);
                let child_index = parent.child_index();

                match parent.parent() {
                    Some(grandparent) => {
                        {
                            let grandparent = self.get_item(grandparent);
                            if grandparent.child_count() > child_index + 1 {
                                return Some(grandparent.child(child_index + 1));
                            }
                        }
                        return self.next_uncle(parent.index());
                    }
                    None => {
                        let roots = self.get_roots();
                        if roots.len() > child_index + 1 {
                            return Some(roots[child_index + 1]);
                        }
                        None
                    }
                }
            }
            None => None,
        }
    }

    fn prev_uncle(&self, ix: Index) -> Option<Index> {
        let item = self.get_item(ix);
        match item.parent() {
            Some(parent) => {
                let parent = self.get_item(parent);
                let child_index = parent.child_index();

                match parent.parent() {
                    Some(grandparent) => {
                        {
                            let grandparent = self.get_item(grandparent);
                            if child_index > 0 {
                                return Some(grandparent.child(child_index - 1));
                            }
                        }
                        return self.prev_uncle(parent.index());
                    }
                    None => {
                        let roots = self.get_roots();
                        if child_index > 0 {
                            return Some(roots[child_index - 1]);
                        }
                        None
                    }
                }
            }
            None => None,
        }
    }
}

pub struct TreeView<T: TreeModel> {
    model: T,

    is_collapsed: std::collections::HashSet<Index>,

    focus: Option<Index>,
    focused_row: Option<usize>,

    on_item_focus: Option<Box<dyn Fn(&mut T, Index) -> ()>>,
}

impl<T: TreeModel> TreeView<T> {
    pub fn new(model: T) -> Self {
        Self {
            model,
            is_collapsed: std::collections::HashSet::new(),
            focus: None,
            focused_row: None,
            on_item_focus: None,
        }
    }

    pub fn set_on_item_focus_callback(&mut self, callback: Box<dyn Fn(&mut T, Index) -> ()>) {
        self.on_item_focus = Some(callback);
    }

    fn focus_first(&mut self) {
        self.focus = self.model.get_first();
    }

    fn move_cursor_down(&mut self) {
        let ix = match self.focus {
            Some(focus) => focus,
            None => return self.focus_first(),
        };

        let item = self.model.get_item(ix);

        if item.child_count() == 0 || self.is_collapsed(ix) {
            if let Some(ix) = self
                .model
                .next_sibling(ix)
                .or_else(|| self.model.next_uncle(ix))
            {
                self.focus = Some(ix)
            }
        } else {
            self.focus = Some(item.child(0));
        }

        if self.focus != Some(ix) {
            if let Some(callback) = &self.on_item_focus {
                callback(&mut self.model, ix);
            }
        }
    }

    fn move_cursor_up(&mut self) {
        let ix = match self.focus {
            Some(focus) => focus,
            None => return self.focus_first(),
        };

        let item = self.model.get_item(ix);
        if let Some(ix) = self.model.prev_sibling(ix).or_else(|| item.parent()) {
            self.focus = Some(ix);
            if let Some(callback) = &self.on_item_focus {
                callback(&mut self.model, ix);
            }
        }
    }

    fn is_collapsed(&self, ix: Index) -> bool {
        self.is_collapsed.contains(&ix)
    }

    fn has_children(&self, ix: Index) -> bool {
        self.model.get_item(ix).child_count() > 0
    }

    fn get_item(&self, ix: Index) -> &T::Data {
        self.model.get_item(ix)
    }

    fn toggle_collapse(&mut self, ix: Index) {
        if !self.is_collapsed.remove(&ix) {
            self.is_collapsed.insert(ix);
        }
    }

    fn render_rows(&mut self, ix: Index, level: usize, target: &mut Vec<String>) {
        let indent = unsafe { String::from_utf8_unchecked(vec![b' '; level]) };

        let item = self.get_item(ix);

        let child_count = item.child_count();

        let mut is_collapsed = false;

        let indicator = if child_count > 0 {
            if self.is_collapsed.contains(&ix) {
                is_collapsed = true;
                log::debug!("render collapsed");
                "⏵ "
            } else {
                "⏷ "
            }
        } else {
            ""
        };

        target.push(format!("{indent}{indicator}{}", item.render()));

        if Some(ix) == self.focus {
            self.focused_row = Some(target.len() - 1);
        }

        if is_collapsed {
            return;
        }

        for row in 0..child_count {
            let child = self.get_item(ix).child(row);
            self.render_rows(child, level + 2, target);
        }
    }
}

impl<T: TreeModel + 'static> Component for TreeView<T> {
    fn render(
        &mut self,
        area: helix_view::graphics::Rect,
        surface: &mut tui::buffer::Buffer,
        cx: &mut crate::compositor::Context,
    ) {
        // -- Render the frame:
        // clear area
        let background = cx.editor.theme.get("ui.background");
        let text = cx.editor.theme.get("ui.text");
        surface.clear_with(area, background);

        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        let margin = Margin::horizontal(1);
        let inner = inner.inner(&margin);
        block.render(area, surface);

        let mut rows = Vec::new();

        let mut index = 0;

        self.focused_row = None;

        loop {
            let roots = self.model.get_roots();
            if roots.len() <= index {
                break;
            }

            let root = roots[index];
            self.render_rows(root, 0, &mut rows);

            index = index + 1;
        }

        for (row, line) in rows.iter().enumerate() {
            if row >= inner.height as usize {
                break;
            }

            let mut style = Style::default();

            if Some(row) == self.focused_row {
                std::mem::swap(&mut style.fg, &mut style.bg);
                style.add_modifier(Modifier::REVERSED);
                style.fg = Some(helix_view::theme::Color::Black);
                style.bg = Some(helix_view::theme::Color::White);

                log::debug!("highlight");
            }

            surface.set_string(inner.x, inner.y + row as u16, line, style)
        }
    }

    fn handle_event(
        &mut self,
        event: &helix_view::input::Event,
        _ctx: &mut crate::compositor::Context,
    ) -> EventResult {
        let event = match event {
            Event::Key(event) => event,
            _ => return EventResult::Ignored(None),
        };

        let close_fn = EventResult::Consumed(Some(Box::new(|compositor: &mut Compositor, _cx| {
            // remove the layer
            compositor.last_picker = compositor.pop();
        })));

        match event {
            key!('q') => close_fn,
            key!(Up) => {
                self.move_cursor_up();
                {
                    let ix = self.focus.unwrap();
                    let item = self.get_item(ix);

                    log::debug!(
                        "up index={} row={:?} collapsed={} child_count={} child_index={}",
                        ix.0,
                        self.focused_row,
                        self.is_collapsed(ix),
                        item.child_count(),
                        item.child_index()
                    );
                }
                EventResult::Consumed(None)
            }
            key!(Down) => {
                self.move_cursor_down();
                {
                    let ix = self.focus.unwrap();
                    let item = self.get_item(ix);

                    log::debug!(
                        "up index={} row={:?} collapsed={} child_count={} child_index={}",
                        ix.0,
                        self.focused_row,
                        self.is_collapsed(ix),
                        item.child_count(),
                        item.child_index()
                    );
                }
                EventResult::Consumed(None)
            }
            key!(Enter) => {
                if let Some(focused) = self.focus {
                    log::debug!(
                        "collapse index={} row={:?} collapsed={}",
                        focused.0,
                        self.focused_row,
                        self.is_collapsed(self.focus.unwrap())
                    );
                    self.toggle_collapse(focused);
                }
                EventResult::Consumed(None)
            }
            _ => EventResult::Ignored(None),
        }
    }

    fn should_update(&self) -> bool {
        true
    }

    fn cursor(
        &self,
        _area: helix_view::graphics::Rect,
        _ctx: &helix_view::Editor,
    ) -> (
        Option<helix_core::Position>,
        helix_view::graphics::CursorKind,
    ) {
        (None, helix_view::graphics::CursorKind::Hidden)
    }

    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        Some(viewport)
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn id(&self) -> Option<&'static str> {
        None
    }
}
