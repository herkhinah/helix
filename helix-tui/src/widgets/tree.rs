use crate::{buffer::Buffer, widgets::Widget};
use helix_view::{
    graphics::Rect,
    theme::{Modifier, Style},
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RowRef(usize);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChildRef(usize);

impl std::ops::Deref for ChildRef {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::Deref for RowRef {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::Add<usize> for ChildRef {
    type Output = ChildRef;

    fn add(self, rhs: usize) -> Self::Output {
        ChildRef(*self + rhs)
    }
}

impl std::ops::Sub<usize> for ChildRef {
    type Output = ChildRef;

    fn sub(self, rhs: usize) -> Self::Output {
        ChildRef(*self - rhs)
    }
}

pub trait Row<const COLUMNS: usize> {
    type Model;

    fn render_cell(
        &self,
        model: &Self::Model,
        column: usize,
        row: RowRef,
        area: Rect,
        but: &mut Buffer,
    ) -> Rect;
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Generation(usize);

impl Generation {
    pub fn inc(&mut self) {
        self.0 += 1;
    }
}

pub struct TreeModel<const COLUMNS: usize, RowData: Row<COLUMNS>> {
    pub data: Vec<RowData>,

    pub roots: Vec<RowRef>,

    pub collapsed: Vec<bool>,
    pub parent: Vec<Option<RowRef>>,
    pub child_ref: Vec<ChildRef>,
    pub children: Vec<Vec<RowRef>>,

    pub focus: Focus<RowData, COLUMNS>,
}

pub struct Focus<RowData, const COLUMNS: usize>
where
    RowData: Row<COLUMNS>,
{
    pub row: Option<RowRef>,
    pub callback: Option<Box<dyn Fn(&TreeModel<COLUMNS, RowData>, RowRef) -> ()>>,
    pub style: Style,
}

impl<RowData: Row<COLUMNS>, const COLUMNS: usize> std::default::Default
    for Focus<RowData, COLUMNS>
{
    fn default() -> Self {
        Self {
            row: None,
            callback: None,
            style: Style::default().add_modifier(Modifier::REVERSED),
        }
    }
}

impl<const COLUMNS: usize, RowData: Row<COLUMNS, Model = TreeModel<COLUMNS, RowData>>>
    TreeModel<COLUMNS, RowData>
{
    fn next_sibling(&self, ix: RowRef) -> Option<RowRef> {
        let child_index = self.child_ref[*ix];
        match self.parent[*ix] {
            Some(parent_ix) => {
                let children = &self.children[*parent_ix];
                if children.len() > *child_index + 1 {
                    return Some(children[*child_index + 1]);
                }
                None
            }
            None => {
                if self.roots.len() > *child_index + 1 {
                    return Some(self.roots[*child_index + 1]);
                }
                None
            }
        }
    }

    fn prev_sibling(&self, ix: RowRef) -> Option<RowRef> {
        let child_index = self.child_ref[*ix];
        match self.parent[*ix] {
            Some(parent_ix) => {
                if *child_index > 0 {
                    return Some(self.children[*parent_ix][*child_index - 1]);
                }
                None
            }
            None => {
                if *child_index > 0 {
                    return Some(self.roots[*child_index]);
                }
                None
            }
        }
    }

    fn next_uncle(&self, ix: RowRef) -> Option<RowRef> {
        match self.parent[*ix] {
            Some(parent_ix) => {
                let child_index = self.child_ref[*parent_ix];

                match self.parent[*parent_ix] {
                    Some(grandparent_ix) => {
                        {
                            let grandparent_children = &self.children[*grandparent_ix];
                            if grandparent_children.len() > *child_index + 1 {
                                return Some(grandparent_children[*child_index + 1]);
                            }
                        }
                        return self.next_uncle(parent_ix);
                    }
                    None => {
                        if self.roots.len() > *child_index + 1 {
                            return Some(self.roots[*child_index + 1]);
                        }
                        None
                    }
                }
            }
            None => None,
        }
    }

    fn prev_uncle(&self, ix: RowRef) -> Option<RowRef> {
        match self.parent[*ix] {
            Some(parent_ix) => {
                let child_index = self.child_ref[*parent_ix];

                match self.parent[*parent_ix] {
                    Some(grandparent_ix) => {
                        {
                            if *child_index > 0 {
                                return Some(self.children[*grandparent_ix][*child_index - 1]);
                            }
                        }
                        return self.prev_uncle(parent_ix);
                    }
                    None => {
                        if *child_index > 0 {
                            return Some(self.roots[*child_index - 1]);
                        }
                        None
                    }
                }
            }
            None => None,
        }
    }

    pub fn on_focus_callback(&mut self, callback: Box<dyn Fn(&Self, RowRef) -> ()>) {
        self.focus.callback = Some(callback);
    }

    fn focus_first(&mut self) {
        self.focus.row = self.roots.get(0).map(|x| *x)
    }

    fn move_cursor_down(&mut self) {
        let ix = match self.focus.row {
            Some(ix) => ix,
            None => return self.focus_first(),
        };

        let children = &self.children[*ix];

        if children.len() == 0 || self.collapsed[*ix] {
            if let Some(ix) = self.next_sibling(ix).or_else(|| self.next_uncle(ix)) {
                self.focus.row = Some(ix);
            }
        } else {
            self.focus.row = Some(children[0]);
        }

        if self.focus.row != Some(ix) {
            if let Some(callback) = &self.focus.callback {
                callback(self, ix);
            }
        }
    }

    fn move_cursor_up(&mut self) {
        let ix = match self.focus.row {
            Some(ix) => ix,
            None => return self.focus_first(),
        };

        if let Some(ix) = self.prev_sibling(ix).or_else(|| self.parent[*ix]) {
            self.focus.row = Some(ix);
            if let Some(callback) = &self.focus.callback {
                callback(self, ix);
            }
        }
    }

    fn is_collapsed(&self, ix: RowRef) -> bool {
        self.collapsed[*ix]
    }

    fn has_children(&self, ix: RowRef) -> bool {
        !self.children[*ix].is_empty()
    }

    fn toggle_collapse(&mut self, ix: RowRef) {
        self.collapsed[*ix] ^= true;
    }

    fn render_col(&self, column: usize, row: RowRef, mut area: Rect, buf: &mut Buffer) -> Rect {
        let children = &self.children[*row];

        let mut is_collapsed = false;

        let style = if self.focus.row == Some(row) {
            self.focus.style
        } else {
            Style::default()
        };

        if children.len() > 0 {
            if self.collapsed[*row] {
                is_collapsed = true;
                if column == 0 {
                    buf.set_string(area.x, area.y, "⏵ ", style);
                }
            } else {
                if column == 0 {
                    buf.set_string(area.x, area.y, "⏷ ", style);
                }
            }
            area = area.clip_left(2);
        }

        let mut area_used_total = self.data[*row].render_cell(self, column, row, area, buf);

        if is_collapsed {
            return area_used_total;
        }

        for row in children {
            let area_used = self.render_col(column, *row, area, buf);
            let area = area.clip_top(area_used.height);
            area_used_total = area_used_total.union(area_used);
        }

        area_used_total
    }
}

impl<const COLUMNS: usize, RowData: Row<COLUMNS, Model = TreeModel<COLUMNS, RowData>>> Widget
    for &TreeModel<COLUMNS, RowData>
{
    fn render(self, mut area: helix_view::graphics::Rect, surface: &mut Buffer) {
        let mut index = 0;

        for col in 0..COLUMNS {
            let mut col_width = 0u16;

            for row in &self.roots {
                let root = self.roots[index];
                let drawn_area = self.render_col(col, *row, area, surface);

                area = area.clip_top(drawn_area.height);
                col_width = std::cmp::max(col_width, drawn_area.width);

                index = index + 1;
            }

            area = area.clip_left(col_width + 1);
        }
    }
}
