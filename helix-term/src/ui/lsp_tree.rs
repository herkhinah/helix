use std::pin::{pin, Pin};

use helix_lsp::lsp;

use super::tree::*;

struct Item<'a> {
    item: Pin<&'a lsp::DocumentSymbol>,
    row: usize,

    children: Vec<Index>,

    index: Index,
    parent: Option<Index>,
}

impl<'a> TreeItem for Item<'a> {
    type Data = Pin<&'a lsp::DocumentSymbol>;

    fn child(&self, row: usize) -> Index {
        self.children[row]
    }

    fn child_count(&self) -> usize {
        self.children.len()
    }

    fn data(&self, column: usize) -> Self::Data {
        self.item.as_ref()
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
    symbols: Pin<Vec<lsp::DocumentSymbol>>,

    items: Vec<Pin<Item<'a>>>,
}

impl<'a, 'b: 'a> LspTreeModel<'a> {
    fn initialize(&'b mut self) {
        fn traverse_children<'a, 'b: 'a, 'c: 'b>(
            store: &'a mut Vec<Item<'b>>,
            item: Option<Item<'b>>,
            next_row: &mut usize,
            parent: Option<Index>,
            children: &'c Vec<lsp::DocumentSymbol>,
        ) {
            let mut index: Option<usize> = None;
            if let Some(item) = item {
                index = Some(store.len());
                store.as_mut().push(item);
            }

            let mut indices: Vec<Index> = Vec::with_capacity(children.len());

            for item in children {
                let index = store.len();
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

                traverse_children(store, Some(item_), next_row, Some(Index(index)), &children);
            }

            if let Some(index) = index {
                store[index].children = indices;
            }
        }

        let mut next_row = 0usize;

        traverse_children(&mut self.items, None, &mut next_row, None, &self.symbols);
    }

    pub fn new(symbols: Vec<lsp::DocumentSymbol>) -> Self {
        let mut model = Self {
            symbols,
            items: Pin::new(Vec::new()),
        };

        Self::initialize(&mut model);
        model
    }
}
