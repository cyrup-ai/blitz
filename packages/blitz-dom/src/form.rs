use core::str::FromStr;
use std::fmt::Display;

use blitz_text::Edit;
use blitz_traits::navigation::NavigationOptions;
use markup5ever::{LocalName, local_name};

use crate::{
    BaseDocument, ElementData,
    node::FileData,
    traversal::{AncestorTraverser, TreeTraverser},
};

/// Determine document encoding from meta elements per HTML5 specification
/// https://html.spec.whatwg.org/multipage/semantics.html#determining-the-character-encoding
fn determine_document_encoding(doc: &BaseDocument) -> Option<&'static str> {
    // Look for meta elements in the document head
    for node_id in TreeTraverser::new(doc) {
        let Some(node) = doc.get_node(node_id) else {
            continue;
        };
        let Some(element) = node.element_data() else {
            continue;
        };

        if element.name.local == local_name!("meta") {
            // Check for charset attribute: <meta charset="UTF-8">
            if let Some(charset) = element.attr(local_name!("charset")) {
                return Some(normalize_encoding_name(charset));
            }

            // Check for http-equiv content: <meta http-equiv="Content-Type" content="text/html; charset=UTF-8">
            if element.attr(local_name!("http-equiv")).as_deref() == Some("Content-Type") {
                if let Some(content) = element.attr(local_name!("content")) {
                    // Parse charset from content attribute
                    if let Some(charset_start) = content.to_lowercase().find("charset=") {
                        let charset_value = &content[charset_start + 8..];
                        // Extract charset value, handling quotes and semicolons
                        let charset = charset_value
                            .trim_start_matches(['"', '\''])
                            .split([';', '"', '\'', ' ', '\t'])
                            .next()
                            .unwrap_or("")
                            .trim();
                        if !charset.is_empty() {
                            return Some(normalize_encoding_name(charset));
                        }
                    }
                }
            }
        }
    }

    None
}

/// Normalize encoding names to standard labels per HTML5 specification
/// https://encoding.spec.whatwg.org/#names-and-labels
fn normalize_encoding_name(charset: &str) -> &'static str {
    match charset.to_lowercase().as_str() {
        "utf-8" | "utf8" | "unicode-1-1-utf-8" => "UTF-8",
        "iso-8859-1" | "latin1" | "cp1252" | "windows-1252" => "windows-1252",
        "ascii" | "us-ascii" | "ansi_x3.4-1968" => "windows-1252", /* ASCII is treated as windows-1252 */
        "utf-16" | "utf-16le" | "unicode" => "UTF-16LE",
        "utf-16be" => "UTF-16BE",
        "iso-8859-2" | "latin2" => "ISO-8859-2",
        "iso-8859-3" | "latin3" => "ISO-8859-3",
        "iso-8859-4" | "latin4" => "ISO-8859-4",
        "iso-8859-5" | "cyrillic" => "ISO-8859-5",
        "iso-8859-6" | "arabic" => "ISO-8859-6",
        "iso-8859-7" | "greek" => "ISO-8859-7",
        "iso-8859-8" | "hebrew" => "ISO-8859-8",
        "iso-8859-9" | "latin5" => "ISO-8859-9",
        "iso-8859-10" | "latin6" => "ISO-8859-10",
        "iso-8859-13" | "latin7" => "ISO-8859-13",
        "iso-8859-14" | "latin8" => "ISO-8859-14",
        "iso-8859-15" | "latin9" => "ISO-8859-15",
        "iso-8859-16" | "latin10" => "ISO-8859-16",
        "windows-1250" | "cp1250" => "windows-1250",
        "windows-1251" | "cp1251" => "windows-1251",
        "windows-1253" | "cp1253" => "windows-1253",
        "windows-1254" | "cp1254" => "windows-1254",
        "windows-1255" | "cp1255" => "windows-1255",
        "windows-1256" | "cp1256" => "windows-1256",
        "windows-1257" | "cp1257" => "windows-1257",
        "windows-1258" | "cp1258" => "windows-1258",
        "macintosh" | "mac" => "macintosh",
        "x-mac-cyrillic" | "mac-cyrillic" => "x-mac-cyrillic",
        _ => "UTF-8", // Default fallback
    }
}

impl BaseDocument {
    /// Resets the form owner for a given node by either using an explicit form attribute
    /// or finding the nearest ancestor form element
    ///
    /// # Arguments
    /// * `node_id` - The ID of the node whose form owner needs to be reset
    ///
    /// <https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#reset-the-form-owner>
    pub fn reset_form_owner(&mut self, node_id: usize) {
        let node = &self.nodes[node_id];
        let Some(element) = node.element_data() else {
            return;
        };

        // First try explicit form attribute
        let final_owner_id = element
            .attr(local_name!("form"))
            .and_then(|owner| self.nodes_to_id.get(owner))
            .copied()
            .filter(|owner_id| {
                self.get_node(*owner_id)
                    .is_some_and(|node| node.data.is_element_with_tag_name(&local_name!("form")))
            })
            .or_else(|| {
                AncestorTraverser::new(self, node_id).find(|ancestor_id| {
                    self.nodes[*ancestor_id]
                        .data
                        .is_element_with_tag_name(&local_name!("form"))
                })
            });

        if let Some(final_owner_id) = final_owner_id {
            self.controls_to_form.insert(node_id, final_owner_id);
        }
    }

    /// Submits a form with the given form node ID and submitter node ID
    ///
    /// # Arguments
    /// * `node_id` - The ID of the form node to submit
    /// * `submitter_id` - The ID of the node that triggered the submission
    ///
    /// <https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#form-submission-algorithm>
    pub fn submit_form(&self, node_id: usize, submitter_id: usize) {
        let node = &self.nodes[node_id];
        let Some(element) = node.element_data() else {
            return;
        };

        let entry = construct_entry_list(self, node_id, submitter_id, None);

        let method = get_form_attr(
            self,
            element,
            local_name!("method"),
            submitter_id,
            local_name!("formmethod"),
        )
        .and_then(|method| method.parse::<FormMethod>().ok())
        .unwrap_or(FormMethod::Get);

        let action = get_form_attr(
            self,
            element,
            local_name!("action"),
            submitter_id,
            local_name!("formaction"),
        )
        .unwrap_or_default();

        let mut parsed_action = self.resolve_url(action);

        let scheme = parsed_action.scheme();

        let enctype = get_form_attr(
            self,
            element,
            local_name!("enctype"),
            submitter_id,
            local_name!("formenctype"),
        )
        .and_then(|enctype| enctype.parse::<RequestContentType>().ok())
        .unwrap_or(RequestContentType::FormUrlEncoded);

        let mut post_resource = None;

        match (scheme, method) {
            ("http" | "https" | "data", FormMethod::Get) => {
                let pairs = entry.convert_to_list_of_name_value_pairs();

                let mut query = String::new();
                url::form_urlencoded::Serializer::new(&mut query).extend_pairs(pairs);

                parsed_action.set_query(Some(&query));
            }

            ("http" | "https", FormMethod::Post) => match enctype {
                RequestContentType::FormUrlEncoded => {
                    let pairs = entry.convert_to_list_of_name_value_pairs();
                    let mut body = String::new();
                    url::form_urlencoded::Serializer::new(&mut body).extend_pairs(pairs);
                    post_resource = Some(body.into());
                }
                RequestContentType::MultipartFormData => {
                    #[cfg(feature = "tracing")]
                    tracing::warn!("Multipart Forms are currently not supported");
                    return;
                }
                RequestContentType::TextPlain => {
                    let pairs = entry.convert_to_list_of_name_value_pairs();
                    let body = encode_text_plain(&pairs).into();
                    post_resource = Some(body);
                }
            },
            ("mailto", FormMethod::Get) => {
                let pairs = entry.convert_to_list_of_name_value_pairs();

                parsed_action.query_pairs_mut().extend_pairs(pairs);
            }
            ("mailto", FormMethod::Post) => {
                let pairs = entry.convert_to_list_of_name_value_pairs();
                let body = match enctype {
                    RequestContentType::TextPlain => {
                        let body = encode_text_plain(&pairs);

                        /// https://url.spec.whatwg.org/#default-encode-set
                        const DEFAULT_ENCODE_SET: percent_encoding::AsciiSet =
                            percent_encoding::CONTROLS
                                // Query Set
                                .add(b' ')
                                .add(b'"')
                                .add(b'#')
                                .add(b'<')
                                .add(b'>')
                                // Path Set
                                .add(b'?')
                                .add(b'`')
                                .add(b'{')
                                .add(b'}');

                        // Set body to the result of running UTF-8 percent-encode on body using the default encode set. [URL]
                        percent_encoding::utf8_percent_encode(&body, &DEFAULT_ENCODE_SET)
                            .to_string()
                    }
                    _ => {
                        let mut body = String::new();
                        url::form_urlencoded::Serializer::new(&mut body).extend_pairs(pairs);
                        body
                    }
                };
                let mut query = if let Some(query) = parsed_action.query() {
                    let mut query = query.to_string();
                    query.push('&');
                    query
                } else {
                    String::new()
                };
                query.push_str("body=");
                query.push_str(&body);
                parsed_action.set_query(Some(&query));
            }
            _ => {
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    "Scheme {} with method {:?} is not implemented",
                    scheme,
                    method
                );
                return;
            }
        }

        let navigation_options =
            NavigationOptions::new(parsed_action, enctype.to_string(), self.id())
                .set_document_resource(post_resource);

        self.navigation_provider.navigate_to(navigation_options)
    }

    /// Submits a form with the given form node ID, submitter node ID, and optional click coordinates
    ///
    /// # Arguments
    /// * `node_id` - The ID of the form node to submit
    /// * `submitter_id` - The ID of the node that triggered the submission
    /// * `coords` - Optional click coordinates for image button submissions
    ///
    /// <https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#form-submission-algorithm>
    pub fn submit_form_with_coordinates(
        &self,
        node_id: usize,
        submitter_id: usize,
        coords: Option<(i32, i32)>,
    ) {
        let node = &self.nodes[node_id];
        let Some(element) = node.element_data() else {
            return;
        };

        let entry = construct_entry_list(self, node_id, submitter_id, coords);

        let method = get_form_attr(
            self,
            element,
            local_name!("method"),
            submitter_id,
            local_name!("formmethod"),
        )
        .and_then(|method| method.parse::<FormMethod>().ok())
        .unwrap_or(FormMethod::Get);

        let action = get_form_attr(
            self,
            element,
            local_name!("action"),
            submitter_id,
            local_name!("formaction"),
        )
        .unwrap_or_default();

        let mut parsed_action = self.resolve_url(action);

        let scheme = parsed_action.scheme();

        let enctype = get_form_attr(
            self,
            element,
            local_name!("enctype"),
            submitter_id,
            local_name!("formenctype"),
        )
        .and_then(|enctype| enctype.parse::<RequestContentType>().ok())
        .unwrap_or(RequestContentType::FormUrlEncoded);

        let mut post_resource = None;

        match (scheme, method) {
            ("http" | "https" | "data", FormMethod::Get) => {
                let pairs = entry.convert_to_list_of_name_value_pairs();

                let mut query = String::new();
                url::form_urlencoded::Serializer::new(&mut query).extend_pairs(pairs);

                parsed_action.set_query(Some(&query));
            }

            ("http" | "https", FormMethod::Post) => match enctype {
                RequestContentType::FormUrlEncoded => {
                    let pairs = entry.convert_to_list_of_name_value_pairs();
                    let mut body = String::new();
                    url::form_urlencoded::Serializer::new(&mut body).extend_pairs(pairs);
                    post_resource = Some(body.into());
                }
                RequestContentType::MultipartFormData => {
                    #[cfg(feature = "tracing")]
                    tracing::warn!("Multipart Forms are currently not supported");
                    return;
                }
                RequestContentType::TextPlain => {
                    let pairs = entry.convert_to_list_of_name_value_pairs();
                    let body = encode_text_plain(&pairs).into();
                    post_resource = Some(body);
                }
            },
            ("mailto", FormMethod::Get) => {
                let pairs = entry.convert_to_list_of_name_value_pairs();

                parsed_action.query_pairs_mut().extend_pairs(pairs);
            }
            ("mailto", FormMethod::Post) => {
                let pairs = entry.convert_to_list_of_name_value_pairs();
                let body = match enctype {
                    RequestContentType::TextPlain => {
                        let body = encode_text_plain(&pairs);

                        /// https://url.spec.whatwg.org/#default-encode-set
                        const DEFAULT_ENCODE_SET: percent_encoding::AsciiSet =
                            percent_encoding::CONTROLS
                                // Query Set
                                .add(b' ')
                                .add(b'"')
                                .add(b'#')
                                .add(b'<')
                                .add(b'>')
                                // Path Set
                                .add(b'?')
                                .add(b'`')
                                .add(b'{')
                                .add(b'}');

                        // Set body to the result of running UTF-8 percent-encode on body using the default encode set. [URL]
                        percent_encoding::utf8_percent_encode(&body, &DEFAULT_ENCODE_SET)
                            .to_string()
                    }
                    _ => {
                        let mut body = String::new();
                        url::form_urlencoded::Serializer::new(&mut body).extend_pairs(pairs);
                        body
                    }
                };
                let mut query = if let Some(query) = parsed_action.query() {
                    let mut query = query.to_string();
                    query.push('&');
                    query
                } else {
                    String::new()
                };
                query.push_str("body=");
                query.push_str(&body);
                parsed_action.set_query(Some(&query));
            }
            _ => {
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    "Scheme {} with method {:?} is not implemented",
                    scheme,
                    method
                );
                return;
            }
        }

        let navigation_options =
            NavigationOptions::new(parsed_action, enctype.to_string(), self.id())
                .set_document_resource(post_resource);

        self.navigation_provider.navigate_to(navigation_options)
    }
}

/// Constructs a list of form entries from form controls
///
/// # Arguments
/// * `doc` - Reference to the base document
/// * `form_id` - ID of the form element
/// * `submitter_id` - ID of the element that triggered form submission
/// * `click_coords` - Optional click coordinates for image button submissions
///
/// # Returns
/// Returns an EntryList containing all valid form control entries
///
/// https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#constructing-the-form-data-set
fn construct_entry_list(
    doc: &BaseDocument,
    form_id: usize,
    submitter_id: usize,
    click_coords: Option<(i32, i32)>,
) -> EntryList {
    let mut entry_list = EntryList::new();



    fn datalist_ancestor(doc: &BaseDocument, node_id: usize) -> bool {
        AncestorTraverser::new(doc, node_id).any(|node_id| {
            doc.nodes[node_id]
                .data
                .is_element_with_tag_name(&local_name!("datalist"))
        })
    }

    for control_id in TreeTraverser::new(doc) {
        let Some(node) = doc.get_node(control_id) else {
            continue;
        };
        let Some(element) = node.element_data() else {
            continue;
        };

        // Check if the form owner is same as form_id
        if doc
            .controls_to_form
            .get(&control_id)
            .map(|owner_id| *owner_id != form_id)
            .unwrap_or(true)
        {
            continue;
        }

        let element_type = element.attr(local_name!("type"));

        //  If any of the following are true:
        //   field has a datalist element ancestor;
        //   field is disabled;
        //   field is a button but it is not submitter;
        //   field is an input element whose type attribute is in the Checkbox state and whose checkedness is false; or
        //   field is an input element whose type attribute is in the Radio Button state and whose checkedness is false,
        //  then continue.
        if datalist_ancestor(doc, node.id)
            || element.attr(local_name!("disabled")).is_some()
            || (element.name.local == local_name!("button") && node.id != submitter_id)
            || element.name.local == local_name!("input")
                && ((matches!(element_type, Some("checkbox" | "radio"))
                    && !element.checkbox_input_checked().unwrap_or(false))
                    || matches!(element_type, Some("submit" | "button")))
        {
            continue;
        }

        // If the field element is an input element whose type attribute is in the Image Button state, then:
        if element_type == Some("image") {
            // If the field element is not submitter, then continue.
            if node.id != submitter_id {
                continue;
            }

            // Process image button coordinate submission per HTML spec
            // If the field element has a name attribute specified and its value is not the empty string,
            // let name be that value followed by U+002E (.). Otherwise, let name be the empty string.
            let name_base = element
                .attr(local_name!("name"))
                .and_then(|name| {
                    let name_str: &str = name.as_ref();
                    if name_str.is_empty() {
                        None
                    } else {
                        Some(format!("{}.", name_str))
                    }
                })
                .unwrap_or_else(String::new);

            // Let namex be the concatenation of name and U+0078 (x).
            // Let namey be the concatenation of name and U+0079 (y).
            let namex = format!("{}x", name_base);
            let namey = format!("{}y", name_base);

            // Let (x, y) be the selected coordinate.
            let (x, y) = match click_coords {
                Some((cx, cy)) => {
                    // Use provided click coordinates (from mouse event)
                    (cx, cy)
                }
                None => {
                    // Fallback: calculate element center using layout information
                    let element_center_x = (node.final_layout.size.width / 2.0) as i32;
                    let element_center_y = (node.final_layout.size.height / 2.0) as i32;
                    (element_center_x, element_center_y)
                }
            };

            // Create an entry with namex and x, and append it to entry list.
            // Create an entry with namey and y, and append it to entry list.
            entry_list.0.push(Entry::new_text(&namex, &x.to_string()));
            entry_list.0.push(Entry::new_text(&namey, &y.to_string()));

            // Continue.
            continue;
        }

        //     If either the field element does not have a name attribute specified, or its name attribute's value is the empty string, then continue.
        //     Let name be the value of the field element's name attribute.
        let Some(name) = element
            .attr(local_name!("name"))
            .filter(|str| !str.is_empty())
        else {
            continue;
        };

        // If the field is a form-associated custom element,
        //  then perform the entry construction algorithm given field and entry list,
        //  then continue.
        if element.is_form_associated_custom_element() {
            if let Some(form_value) = element.form_value() {
                entry_list.0.push(Entry::new_text(name, &form_value));
            }
            continue;
        }

        // If the field element is a select element,
        if element.name.local == local_name!("select") {
            // then for each option element in the select element's
            // list of options whose selectedness is true and that is not disabled,
            for child_id in TreeTraverser::new(doc) {
                let Some(child_node) = doc.get_node(child_id) else {
                    continue;
                };
                let Some(child_element) = child_node.element_data() else {
                    continue;
                };

                // Check if this is an option element that's a child of our select
                if child_element.name.local == local_name!("option") {
                    // Check if this option is within our select element by traversing ancestors
                    let mut is_child_of_select = false;
                    for ancestor_id in AncestorTraverser::new(doc, child_id) {
                        if ancestor_id == control_id {
                            is_child_of_select = true;
                            break;
                        }
                    }

                    if !is_child_of_select {
                        continue;
                    }

                    // Check if option is selected and not disabled
                    let is_selected = child_element.attr(local_name!("selected")).is_some();
                    let is_disabled = child_element.attr(local_name!("disabled")).is_some();

                    if is_selected && !is_disabled {
                        // create an entry with name and the value of the option element,
                        let option_value =
                            child_element.attr(local_name!("value")).unwrap_or_else(|| {
                                // If no value attribute, use the text content of the option
                                // TODO: Extract text content from option element
                                ""
                            });
                        // and append it to entry list.
                        entry_list.0.push(Entry::new_text(name, option_value));
                    }
                }
            }
            continue;
        }

        // Otherwise, if the field element is an input element whose type attribute is in the Checkbox state or the Radio Button state, then:
        if element.name.local == local_name!("input")
            && matches!(element_type, Some("checkbox" | "radio"))
        {
            // If the field element has a value attribute specified, then let value be the value of that attribute; otherwise, let value be the string "on".
            let value = element.attr(local_name!("value")).unwrap_or("on");
            //         Create an entry with name and value, and append it to entry list.
            entry_list.0.push(Entry::new_text(name, value));
        }
        // Otherwise, if the field element is an input element whose type attribute is in the File Upload state, then:
        else if element.name.local == local_name!("input") && element_type == Some("file") {
            // Get the files from the input element state
            let empty_files = Vec::new();
            let selected_files = element
                .file_input_data()
                .map(|data| &data.selected_files)
                .unwrap_or(&empty_files);

            if selected_files.is_empty() {
                // If there are no selected files, then create an entry with name and a new File object
                // with an empty name, application/octet-stream as type, and an empty body
                entry_list.0.push(Entry::new_file(name, FileData {
                    name: String::new(),
                    content_type: "application/octet-stream".to_string(),
                    size: 0,
                    data: Vec::new(),
                }));
            } else {
                // Otherwise, for each file in selected files, create an entry with name and a
                // File object representing the file, and append it to entry list.
                for file in selected_files {
                    entry_list.0.push(Entry::new_file(name, file.clone()));
                }
            }
        }
        // Otherwise, if the field element is an input element whose type attribute is in the Hidden state and name is an ASCII case-insensitive match for "_charset_":
        else if element.name.local == local_name!("input")
            && element_type == Some("hidden")
            && name.eq_ignore_ascii_case("_charset_")
        {
            // Let charset be the name of encoding.
            // Determine document encoding from document state or meta elements
            let charset = determine_document_encoding(doc).unwrap_or("UTF-8");
            // Create an entry with name and charset, and append it to entry list.
            entry_list.0.push(Entry::new_text(name, charset));
        }
        // Otherwise, create an entry with name and the value of the field element, and append it to entry list.
        else if let Some(text) = element.text_input_data() {
            // Get text from cosmyc-text Editor by accessing the buffer
            let text_content = text.editor.with_buffer(|buffer| {
                buffer
                    .lines
                    .iter()
                    .map(|line| line.text())
                    .collect::<Vec<_>>()
                    .join("\n")
            });
            entry_list.0.push(Entry::new_text(name, &text_content));
        } else if let Some(value) = element.attr(local_name!("value")) {
            entry_list.0.push(Entry::new_text(name, value));
        }
    }
    entry_list
}

/// Normalizes line endings in a string according to HTML spec
///
/// Converts single CR or LF to CRLF pairs according to HTML form submission requirements
///
/// # Arguments
/// * `input` - The string whose line endings need to be normalized
///
/// # Returns
/// A new string with normalized CRLF line endings
fn normalize_line_endings(input: &str) -> String {
    // Replace every occurrence of U+000D (CR) not followed by U+000A (LF),
    // and every occurrence of U+000A (LF) not preceded by U+000D (CR),
    // in value, by a string consisting of U+000D (CR) and U+000A (LF).

    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(current) = chars.next() {
        match (current, chars.peek()) {
            ('\r', Some('\n')) => {
                result.push_str("\r\n");
                chars.next();
            }
            ('\r' | '\n', _) => {
                result.push_str("\r\n");
            }
            _ => result.push(current),
        }
    }

    result
}

fn get_form_attr<'a>(
    doc: &'a BaseDocument,
    form: &'a ElementData,
    form_local: impl PartialEq<LocalName>,
    submitter_id: usize,
    submitter_local: impl PartialEq<LocalName>,
) -> Option<&'a str> {
    get_submitter_attr(doc, submitter_id, submitter_local).or_else(|| form.attr(form_local))
}

fn get_submitter_attr(
    doc: &BaseDocument,
    submitter_id: usize,
    local_name: impl PartialEq<LocalName>,
) -> Option<&str> {
    doc.get_node(submitter_id)
        .and_then(|node| node.element_data())
        .and_then(|element_data| {
            if element_data.name.local == local_name!("button")
                && element_data.attr(local_name!("type")) == Some("submit")
            {
                element_data.attr(local_name)
            } else {
                None
            }
        })
}
/// Encodes form data as text/plain according to HTML spec
///
/// # Arguments
/// * `input` - Slice of name-value pairs to encode
///
/// # Returns
/// A string with the encoded form data
///
/// https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#text/plain-encoding-algorithm
fn encode_text_plain(input: &[(String, String)]) -> String {
    let mut out = String::new();
    for (name, value) in input {
        out.push_str(name);
        out.push('=');
        out.push_str(value);
        out.push_str("\r\n");
    }
    out
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum FormMethod {
    Get,
    Post,
    Dialog,
}
impl FromStr for FormMethod {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "get" => FormMethod::Get,
            "post" => FormMethod::Post,
            "dialog" => FormMethod::Dialog,
            _ => return Err(()),
        })
    }
}

/// Supported content types for HTTP requests
#[derive(Debug, Clone)]
pub enum RequestContentType {
    /// application/x-www-form-urlencoded
    FormUrlEncoded,
    /// multipart/form-data
    MultipartFormData,
    /// text/plain
    TextPlain,
}

impl FromStr for RequestContentType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "application/x-www-form-urlencoded" => RequestContentType::FormUrlEncoded,
            "multipart/form-data" => RequestContentType::MultipartFormData,
            "text/plain" => RequestContentType::TextPlain,
            _ => return Err(()),
        })
    }
}

impl Display for RequestContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestContentType::FormUrlEncoded => write!(f, "application/x-www-form-urlencoded"),
            RequestContentType::MultipartFormData => write!(f, "multipart/form-data"),
            RequestContentType::TextPlain => write!(f, "text/plain"),
        }
    }
}

/// A list of form entries used for form submission
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EntryList(pub Vec<Entry>);
impl EntryList {
    /// Creates a new empty EntryList
    pub fn new() -> Self {
        EntryList(Vec::new())
    }

    /// Converts the entry list to a vector of name-value pairs with normalized line endings
    pub fn convert_to_list_of_name_value_pairs(&self) -> Vec<(String, String)> {
        self.0
            .iter()
            .map(|entry| {
                let name = normalize_line_endings(&entry.name);
                let value = match &entry.value {
                    EntryValue::Text(text) => normalize_line_endings(text),
                    EntryValue::File(file_data) => {
                        // For files, use filename as the form value
                        normalize_line_endings(&file_data.name)
                    }
                };
                (name, value)
            })
            .collect()
    }
}

/// Entry value type for form submission
#[derive(Debug, Clone, PartialEq)]
pub enum EntryValue {
    Text(String),
    File(FileData),
}

impl Default for EntryValue {
    fn default() -> Self {
        EntryValue::Text(String::new())
    }
}

/// A single form entry consisting of a name and value
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Entry {
    pub name: String,
    pub value: EntryValue,
}

impl Entry {
    pub fn new_text(name: &str, value: &str) -> Self {
        Self {
            name: name.to_string(),
            value: EntryValue::Text(value.to_string()),
        }
    }
    
    pub fn new_file(name: &str, file_data: FileData) -> Self {
        Self {
            name: name.to_string(),
            value: EntryValue::File(file_data),
        }
    }
}
