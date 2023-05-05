use helix_lsp::lsp;

use crate::{
    commands::Context,
    ui::{overlay::overlayed, tree::TreeView},
};

use super::tree::*;

struct Item {
    item: lsp::DocumentSymbol,
    children: Vec<Index>,
    ix: Index,
    child_ix: usize,
    parent: Option<Index>,
}

impl TreeItem for Item {
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

    fn parent(&self) -> Option<Index> {
        self.parent
    }

    fn render(&self) -> &str {
        self.item.name.as_str()
    }

    fn child_index(&self) -> usize {
        self.child_ix
    }

    fn index(&self) -> Index {
        self.ix
    }
}

struct LspTreeModel {
    pub roots: Vec<Index>,
    pub lsp_items: Vec<Item>,
}

impl LspTreeModel {
    pub fn new(symbols: Vec<lsp::DocumentSymbol>) -> Self {
        log::debug!("symbols: {:?}", symbols);

        fn tr2(
            lsp_items: &mut Vec<Item>,
            mut node: lsp::DocumentSymbol,
            parent: Option<Index>,
            child_ix: usize,
        ) -> Index {
            let index = lsp_items.len();
            let mut children = Vec::new();

            if let Some(children_) = &mut node.children {
                std::mem::swap(&mut children, children_);
            }

            lsp_items.push(Item {
                item: node,
                children: Vec::new(),
                ix: Index(index),
                child_ix,
                parent,
            });

            let mut children: Vec<Index> = children
                .into_iter()
                .enumerate()
                .map(|(child_ix, child)| tr2(lsp_items, child, Some(Index(index)), child_ix))
                .collect();

            std::mem::swap(&mut children, &mut lsp_items[index].children);

            Index(index)
        }

        let mut items = Vec::new();
        let roots = symbols
            .into_iter()
            .enumerate()
            .map(|(child_ix, item)| tr2(&mut items, item, None, child_ix))
            .collect();

        Self {
            lsp_items: items,
            roots,
        }
    }
}

impl TreeModel for LspTreeModel {
    type Data = Item;

    fn get_item(&self, ix: Index) -> &Self::Data {
        &self.lsp_items[*ix]
    }

    fn parent(&self, ix: &Index) -> Option<Index> {
        self.lsp_items[**ix].parent
    }

    fn row_count(&self) -> usize {
        self.lsp_items.len()
    }

    fn column_count(&self) -> usize {
        1
    }

    fn get_roots(&self) -> &[Index] {
        &self.roots
    }
}

pub fn tree_symbol_picker(cx: &mut Context) {
    fn nested_to_flat(
        list: &mut Vec<lsp::SymbolInformation>,
        file: &lsp::TextDocumentIdentifier,
        symbol: lsp::DocumentSymbol,
    ) {
        #[allow(deprecated)]
        list.push(lsp::SymbolInformation {
            name: symbol.name,
            kind: symbol.kind,
            tags: symbol.tags,
            deprecated: symbol.deprecated,
            location: lsp::Location::new(file.uri.clone(), symbol.selection_range),
            container_name: None,
        });
        for child in symbol.children.into_iter().flatten() {
            nested_to_flat(list, file, child);
        }
    }
    let doc = doc!(cx.editor);
    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => {
            cx.editor
                .set_status("Language server not active for current buffer");
            return;
        }
    };

    let current_url = doc.url();
    let offset_encoding = language_server.offset_encoding();

    let future = match language_server.document_symbols(doc.identifier()) {
        Some(future) => future,
        None => {
            cx.editor
                .set_error("Language server does not support document symbols");
            return;
        }
    };

    cx.callback(
        future,
        move |editor, compositor, response: Option<lsp::DocumentSymbolResponse>| {
            if let Some(lsp::DocumentSymbolResponse::Nested(symbols)) = response {
                log::debug!("tree");
                // lsp has two ways to represent symbols (flat/nested)
                // convert the nested variant to flat, so that we have a homogeneous list
                let mut model = LspTreeModel::new(symbols);

                let picker: TreeView<LspTreeModel> = TreeView::new(model);
                compositor.push(Box::new(overlayed(picker)))
            } else {
                log::debug!("flat");
            }
        },
    );
}
