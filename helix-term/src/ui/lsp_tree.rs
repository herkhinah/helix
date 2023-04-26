use std::pin::{pin, Pin};

use helix_lsp::lsp;

use super::tree::*;

struct Item<'a> {
    item: &'a lsp::DocumentSymbol,
    row: usize,

    children: Vec<Index>,

    index: Index,
    parent: Option<Index>,
}

impl<'a> TreeItem for Item<'a> {
    type Data = Index;

    fn child(&self, row: usize) -> Index {
        self.children[row]
    }

    fn child_count(&self) -> usize {
        self.children.len()
    }

    fn data(&self, column: usize) -> Self::Data {
        self.children[column]
    }

    fn row(&self) -> usize {
        self.row
    }

    fn parent(&self) -> Option<Index> {
        self.parent
    }

    fn render(&self) -> &str {
        self.item.name.as_str()
    }
}

struct LspTreeModel<'a> {
    symbols: Vec<lsp::DocumentSymbol>,

    lsp_items: Vec<Item<'a>>,
}

impl<'a> LspTreeModel<'a> {
    pub fn new(symbols: Vec<lsp::DocumentSymbol>) -> Self {
        Self {
            symbols,
            lsp_items: Vec::new(),
        }
    }

    pub fn initialize(&'a mut self) {
        fn traverse_children<'a, 'b: 'a, 'c: 'b>(
            lsp_items: &'a mut Vec<Item<'b>>,
            item: Option<Item<'b>>,
            next_row: &mut usize,
            parent: Option<Index>,
            children: &'c Vec<lsp::DocumentSymbol>,
        ) {
            let mut index: Option<usize> = None;
            if let Some(item) = item {
                index = Some(lsp_items.len());
                lsp_items.push(item);
            }

            let mut indices: Vec<Index> = Vec::with_capacity(children.len());

            for item in children {
                let index = lsp_items.len();
                indices.push(Index(index));

                let row = *next_row;

                *next_row += 1;
                let item_ = Item {
                    item,
                    row,
                    index: Index(index),
                    parent,
                    children: Vec::new(),
                };

                let children = match &item.children {
                    Some(children) => children,
                    None => continue,
                };

                traverse_children(
                    lsp_items,
                    Some(item_),
                    next_row,
                    Some(Index(index)),
                    children,
                );
            }

            if let Some(index) = index {
                lsp_items[index].children = indices;
            }
        }

        let mut next_row = 0usize;

        let Self { symbols, lsp_items } = self;

        traverse_children(lsp_items, None, &mut next_row, None, symbols);
    }
}
