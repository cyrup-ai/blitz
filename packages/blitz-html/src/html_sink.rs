//! An implementation for Html5ever's sink trait, allowing us to parse HTML into a DOM.

use std::borrow::Cow;
use std::cell::{Cell, Ref, RefCell, RefMut};

use blitz_dom::node::Attribute;
use blitz_dom::{BaseDocument, DocumentMutator};
use markup5ever::QualName as BlitzQualName;
use html5ever::ParseOpts;
use html5ever::tokenizer::TokenizerOpts;
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::{
    QualName,
    tendril::{StrTendril, TendrilSink},
    tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink},
};

/// Convert html5ever QualName to blitz-dom QualName
/// Both types have the same structure but come from different markup5ever builds
fn convert_qualname(html5ever_name: html5ever::QualName) -> BlitzQualName {
    BlitzQualName {
        prefix: html5ever_name.prefix,
        ns: html5ever_name.ns,
        local: html5ever_name.local,
    }
}

/// Convert blitz-dom QualName back to html5ever QualName
/// Both types have the same structure but come from different markup5ever builds
fn convert_qualname_back(blitz_name: &BlitzQualName) -> html5ever::QualName {
    html5ever::QualName {
        prefix: blitz_name.prefix.clone(),
        ns: blitz_name.ns.clone(),
        local: blitz_name.local.clone(),
    }
}

/// Convert an html5ever Attribute which uses tendril for its value to a blitz Attribute
/// which uses String.
fn html5ever_to_blitz_attr(attr: html5ever::Attribute) -> Attribute {
    Attribute {
        name: convert_qualname(attr.name),
        value: attr.value.to_string(),
    }
}

pub struct DocumentHtmlParser<'doc> {
    document_mutator: RefCell<DocumentMutator<'doc>>,

    /// Errors that occurred during parsing.
    pub errors: RefCell<Vec<Cow<'static, str>>>,

    /// The document's quirks mode.
    pub quirks_mode: Cell<QuirksMode>,
    pub is_xml: bool,
    
    /// Cache for converted QualNames in elem_name calls
    elem_name_cache: RefCell<Option<html5ever::QualName>>,
}

impl<'doc> DocumentHtmlParser<'doc> {
    #[track_caller]
    /// Get a mutable borrow of the DocumentMutator
    fn mutr(&self) -> RefMut<'_, DocumentMutator<'doc>> {
        self.document_mutator.borrow_mut()
    }
}

impl DocumentHtmlParser<'_> {
    pub fn new(doc: &mut BaseDocument) -> DocumentHtmlParser<'_> {
        DocumentHtmlParser {
            document_mutator: RefCell::new(doc.mutate()),
            errors: RefCell::new(Vec::new()),
            quirks_mode: Cell::new(QuirksMode::NoQuirks),
            is_xml: false,
            elem_name_cache: RefCell::new(None),
        }
    }

    pub fn parse_into_doc<'d>(doc: &'d mut BaseDocument, html: &str) -> &'d mut BaseDocument {
        let mut sink = Self::new(doc);

        let is_xhtml_doc = html.starts_with("<?xml")
            || html.starts_with("<!DOCTYPE") && {
                let first_line = html.lines().next().unwrap();
                first_line.contains("XHTML") || first_line.contains("xhtml")
            };

        if is_xhtml_doc {
            // Parse as XHTML
            sink.is_xml = true;
            html5ever::parse_document(sink, Default::default())
                .from_utf8()
                .read_from(&mut html.as_bytes())
                .unwrap();
        } else {
            // Parse as HTML
            sink.is_xml = false;
            let opts = ParseOpts {
                tokenizer: TokenizerOpts::default(),
                tree_builder: TreeBuilderOpts {
                    exact_errors: false,
                    scripting_enabled: false, // Enables parsing of <noscript> tags
                    iframe_srcdoc: false,
                    drop_doctype: true,
                    quirks_mode: QuirksMode::NoQuirks,
                },
            };
            html5ever::parse_document(sink, opts)
                .from_utf8()
                .read_from(&mut html.as_bytes())
                .unwrap();
        }

        doc
    }
}

impl<'b> TreeSink for DocumentHtmlParser<'b> {
    type Output = ();

    // we use the ID of the nodes in the tree as the handle
    type Handle = usize;

    type ElemName<'a>
        = Ref<'a, QualName>
    where
        Self: 'a;

    fn finish(self) -> Self::Output {
        drop(self.document_mutator.into_inner());
        for error in self.errors.borrow().iter() {
            println!("ERROR: {error}");
        }
    }

    fn parse_error(&self, msg: Cow<'static, str>) {
        self.errors.borrow_mut().push(msg);
    }

    fn get_document(&self) -> Self::Handle {
        0
    }

    fn elem_name<'a>(&'a self, target: &'a Self::Handle) -> Self::ElemName<'a> {
        // Get the blitz-dom QualName and clone it to avoid borrowing issues
        let blitz_qualname = self.document_mutator.borrow()
            .element_name(*target)
            .expect("TreeSink::elem_name called on a node which is not an element!")
            .clone();
        
        let html5ever_qualname = convert_qualname_back(&blitz_qualname);
        
        // Store in cache and return reference
        *self.elem_name_cache.borrow_mut() = Some(html5ever_qualname);
        
        Ref::map(self.elem_name_cache.borrow(), |cache| {
            cache.as_ref().unwrap()
        })
    }

    fn create_element(
        &self,
        name: QualName,
        attrs: Vec<html5ever::Attribute>,
        _flags: ElementFlags,
    ) -> Self::Handle {
        let attrs = attrs.into_iter().map(html5ever_to_blitz_attr).collect();
        self.mutr().create_element(convert_qualname(name), attrs)
    }

    fn create_comment(&self, _text: StrTendril) -> Self::Handle {
        self.mutr().create_comment_node()
    }

    fn create_pi(&self, _target: StrTendril, _data: StrTendril) -> Self::Handle {
        self.mutr().create_comment_node()
    }

    fn append(&self, parent_id: &Self::Handle, child: NodeOrText<Self::Handle>) {
        match child {
            NodeOrText::AppendNode(id) => self.mutr().append_children(*parent_id, &[id]),
            // If content to append is text, first attempt to append it to the last child of parent.
            // Else create a new text node and append it to the parent
            NodeOrText::AppendText(text) => {
                let last_child_id = self.mutr().last_child_id(*parent_id);
                let has_appended = if let Some(id) = last_child_id {
                    self.mutr().append_text_to_node(id, &text).is_ok()
                } else {
                    false
                };
                if !has_appended {
                    let new_child_id = self.mutr().create_text_node(&text);
                    self.mutr().append_children(*parent_id, &[new_child_id]);
                }
            }
        }
    }

    // Note: The tree builder promises we won't have a text node after the insertion point.
    // https://github.com/servo/html5ever/blob/main/rcdom/lib.rs#L338
    fn append_before_sibling(&self, sibling_id: &Self::Handle, new_node: NodeOrText<Self::Handle>) {
        match new_node {
            NodeOrText::AppendNode(id) => self.mutr().insert_nodes_before(*sibling_id, &[id]),
            // If content to append is text, first attempt to append it to the node before sibling_node
            // Else create a new text node and insert it before sibling_node
            NodeOrText::AppendText(text) => {
                let previous_sibling_id = self.mutr().previous_sibling_id(*sibling_id);
                let has_appended = if let Some(id) = previous_sibling_id {
                    self.mutr().append_text_to_node(id, &text).is_ok()
                } else {
                    false
                };
                if !has_appended {
                    let new_child_id = self.mutr().create_text_node(&text);
                    self.mutr()
                        .insert_nodes_before(*sibling_id, &[new_child_id]);
                }
            }
        };
    }

    fn append_based_on_parent_node(
        &self,
        element: &Self::Handle,
        prev_element: &Self::Handle,
        child: NodeOrText<Self::Handle>,
    ) {
        if self.mutr().node_has_parent(*element) {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    fn append_doctype_to_document(
        &self,
        _name: StrTendril,
        _public_id: StrTendril,
        _system_id: StrTendril,
    ) {
        // Ignore. We don't care about the DOCTYPE for now.
    }

    fn get_template_contents(&self, target: &Self::Handle) -> Self::Handle {
        // TODO: implement templates properly. This should allow to function like regular elements.
        *target
    }

    fn same_node(&self, x: &Self::Handle, y: &Self::Handle) -> bool {
        x == y
    }

    fn set_quirks_mode(&self, mode: QuirksMode) {
        self.quirks_mode.set(mode);
    }

    fn add_attrs_if_missing(&self, target: &Self::Handle, attrs: Vec<html5ever::Attribute>) {
        let attrs = attrs.into_iter().map(html5ever_to_blitz_attr).collect();
        self.mutr().add_attrs_if_missing(*target, attrs);
    }

    fn remove_from_parent(&self, target: &Self::Handle) {
        self.mutr().remove_node(*target);
    }

    fn reparent_children(&self, old_parent_id: &Self::Handle, new_parent_id: &Self::Handle) {
        self.mutr()
            .reparent_children(*old_parent_id, *new_parent_id);
    }
}

#[test]
fn parses_some_html() {
    use blitz_dom::DocumentConfig;

    let html = "<!DOCTYPE html><html><body><h1>hello world</h1></body></html>";
    let mut doc = BaseDocument::new(DocumentConfig::default());
    let sink = DocumentHtmlParser::new(&mut doc);

    html5ever::parse_document(sink, Default::default())
        .from_utf8()
        .read_from(&mut html.as_bytes())
        .unwrap();

    doc.print_tree()

    // Now our tree should have some nodes in it
}
