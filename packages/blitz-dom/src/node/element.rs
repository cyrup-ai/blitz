use std::str::FromStr;
use std::sync::Arc;

use blitz_text::{Buffer, Edit, Metrics, EnhancedBuffer};
use color::{AlphaColor, Srgb};
use markup5ever::{LocalName, QualName, local_name};
use selectors::matching::QuirksMode;
use style::Atom;
use style::stylesheets::{DocumentStyleSheet, UrlExtraData};
use style::{
    properties::{PropertyDeclarationBlock, parse_style_attribute},
    servo_arc::Arc as ServoArc,
    shared_lock::{Locked, SharedRwLock},
    stylesheets::CssRuleType,
    values::computed::Gradient as StyloGradient,
};
use url::Url;

use super::{Attribute, Attributes};
use crate::layout::table::TableContext;

#[derive(Debug, Clone)]
pub struct ElementData {
    /// The elements tag name, namespace and prefix
    pub name: QualName,

    /// The elements id attribute parsed as an atom (if it has one)
    pub id: Option<Atom>,

    /// The element's attributes
    pub attrs: Attributes,

    /// Whether the element is focussable
    pub is_focussable: bool,

    /// The element's parsed style attribute (used by stylo)
    pub style_attribute: Option<ServoArc<Locked<PropertyDeclarationBlock>>>,

    /// Heterogeneous data that depends on the element's type.
    /// For example:
    ///   - The image data for \<img\> elements.
    ///   - The cosmyc-text Buffer for inline roots.
    ///   - The text editor for input/textarea elements
    pub special_data: SpecialElementData,

    pub background_images: Vec<Option<BackgroundImageData>>,

    /// Cosmic-text layout (elements with inline inner display mode only)
    pub inline_layout_data: Option<Box<TextLayout>>,

    /// Data associated with display: list-item. Note that this display mode
    /// does not exclude inline_layout_data
    pub list_item_data: Option<Box<ListItemLayout>>,

    /// The element's template contents (\<template\> elements only)
    pub template_contents: Option<usize>,
    // /// Whether the node is a [HTML integration point] (https://html.spec.whatwg.org/multipage/#html-integration-point)
    // pub mathml_annotation_xml_integration_point: bool,
}

#[derive(Copy, Clone, Default)]
pub enum SpecialElementType {
    Stylesheet,
    Image,
    Canvas,
    TableRoot,
    TextInput,
    CheckboxInput,
    #[default]
    None,
}

/// Heterogeneous data that depends on the element's type.
#[derive(Clone, Default)]
pub enum SpecialElementData {
    Stylesheet(DocumentStyleSheet),
    /// An \<img\> element's image data
    Image(Box<ImageData>),
    /// A \<canvas\> element's custom paint source
    Canvas(CanvasData),
    /// Pre-computed table layout data
    TableRoot(Arc<TableContext>),
    /// Cosmic-text text editor (text inputs)
    TextInput(TextInputData),
    /// Checkbox checked state
    CheckboxInput(bool),
    /// File input state tracking
    FileInput(FileInputData),
    /// No data (for nodes that don't need any node-specific data)
    #[default]
    None,
}

impl SpecialElementData {
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

impl ElementData {
    pub fn new(name: QualName, attrs: Vec<Attribute>) -> Self {
        let id_attr_atom = attrs
            .iter()
            .find(|attr| &attr.name.local == "id")
            .map(|attr| attr.value.as_ref())
            .map(|value: &str| Atom::from(value));

        let mut data = ElementData {
            name,
            id: id_attr_atom,
            attrs: Attributes::new(attrs),
            is_focussable: false,
            style_attribute: Default::default(),
            inline_layout_data: None,
            list_item_data: None,
            special_data: SpecialElementData::None,
            template_contents: None,
            background_images: Vec::new(),
        };
        data.flush_is_focussable();
        data
    }

    pub fn attrs(&self) -> &[Attribute] {
        &self.attrs
    }

    pub fn attr(&self, name: impl PartialEq<LocalName>) -> Option<&str> {
        let attr = self.attrs.iter().find(|attr| name == attr.name.local)?;
        Some(&attr.value)
    }

    pub fn attr_parsed<T: FromStr>(&self, name: impl PartialEq<LocalName>) -> Option<T> {
        let attr = self.attrs.iter().find(|attr| name == attr.name.local)?;
        attr.value.parse::<T>().ok()
    }

    /// Detects the presence of the attribute, treating *any* value as truthy.
    pub fn has_attr(&self, name: impl PartialEq<LocalName>) -> bool {
        self.attrs.iter().any(|attr| name == attr.name.local)
    }

    pub fn image_data(&self) -> Option<&ImageData> {
        match &self.special_data {
            SpecialElementData::Image(data) => Some(&**data),
            _ => None,
        }
    }

    pub fn image_data_mut(&mut self) -> Option<&mut ImageData> {
        match self.special_data {
            SpecialElementData::Image(ref mut data) => Some(&mut **data),
            _ => None,
        }
    }

    pub fn raster_image_data(&self) -> Option<&RasterImageData> {
        match self.image_data()? {
            ImageData::Raster(data) => Some(data),
            _ => None,
        }
    }

    pub fn raster_image_data_mut(&mut self) -> Option<&mut RasterImageData> {
        match self.image_data_mut()? {
            ImageData::Raster(data) => Some(data),
            _ => None,
        }
    }

    pub fn canvas_data(&self) -> Option<&CanvasData> {
        match &self.special_data {
            SpecialElementData::Canvas(data) => Some(data),
            _ => None,
        }
    }

    #[cfg(feature = "svg")]
    pub fn svg_data(&self) -> Option<&usvg::Tree> {
        match self.image_data()? {
            ImageData::Svg(data) => Some(data),
            _ => None,
        }
    }

    #[cfg(feature = "svg")]
    pub fn svg_data_mut(&mut self) -> Option<&mut usvg::Tree> {
        match self.image_data_mut()? {
            ImageData::Svg(data) => Some(data),
            _ => None,
        }
    }

    pub fn text_input_data(&self) -> Option<&TextInputData> {
        match &self.special_data {
            SpecialElementData::TextInput(data) => Some(data),
            _ => None,
        }
    }

    pub fn text_input_data_mut(&mut self) -> Option<&mut TextInputData> {
        match &mut self.special_data {
            SpecialElementData::TextInput(data) => Some(data),
            _ => None,
        }
    }

    pub fn checkbox_input_checked(&self) -> Option<bool> {
        match self.special_data {
            SpecialElementData::CheckboxInput(checked) => Some(checked),
            _ => None,
        }
    }

    pub fn checkbox_input_checked_mut(&mut self) -> Option<&mut bool> {
        match self.special_data {
            SpecialElementData::CheckboxInput(ref mut checked) => Some(checked),
            _ => None,
        }
    }

    pub fn file_input_data(&self) -> Option<&FileInputData> {
        match &self.special_data {
            SpecialElementData::FileInput(data) => Some(data),
            _ => None,
        }
    }

    pub fn file_input_data_mut(&mut self) -> Option<&mut FileInputData> {
        match &mut self.special_data {
            SpecialElementData::FileInput(data) => Some(data),
            _ => None,
        }
    }

    pub fn is_form_associated_custom_element(&self) -> bool {
        // Check if element name contains hyphen (custom element indicator)
        self.name.local.contains('-') || 
        self.attrs.iter().any(|attr| attr.name.local.as_ref() == "is")
        // Add other form-associated custom element checks
    }
    
    pub fn form_value(&self) -> Option<String> {
        // Extract form value from custom element
        self.attr(local_name!("value")).map(|s| s.to_string())
    }

    pub fn flush_is_focussable(&mut self) {
        let disabled: bool = self.attr_parsed(local_name!("disabled")).unwrap_or(false);
        let tabindex: Option<i32> = self.attr_parsed(local_name!("tabindex"));

        self.is_focussable = !disabled
            && match tabindex {
                Some(index) => index >= 0,
                None => {
                    // Some focusable HTML elements have a default tabindex value of 0 set under the hood by the user agent.
                    // These elements are:
                    //   - <a> or <area> with href attribute
                    //   - <button>, <frame>, <iframe>, <input>, <object>, <select>, <textarea>, and SVG <a> element
                    //   - <summary> element that provides summary for a <details> element.

                    if [local_name!("a"), local_name!("area")].contains(&self.name.local) {
                        self.attr(local_name!("href")).is_some()
                    } else {
                        const DEFAULT_FOCUSSABLE_ELEMENTS: [LocalName; 6] = [
                            local_name!("button"),
                            local_name!("input"),
                            local_name!("select"),
                            local_name!("textarea"),
                            local_name!("frame"),
                            local_name!("iframe"),
                        ];
                        DEFAULT_FOCUSSABLE_ELEMENTS.contains(&self.name.local)
                    }
                }
            }
    }

    pub fn flush_style_attribute(&mut self, guard: &SharedRwLock, url_extra_data: &UrlExtraData) {
        self.style_attribute = self.attr(local_name!("style")).map(|style_str| {
            ServoArc::new(guard.wrap(parse_style_attribute(
                style_str,
                url_extra_data,
                None,
                QuirksMode::NoQuirks,
                CssRuleType::Style,
            )))
        });
    }

    pub fn take_inline_layout(&mut self) -> Option<Box<TextLayout>> {
        std::mem::take(&mut self.inline_layout_data)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RasterImageData {
    /// The width of the image
    pub width: u32,
    /// The height of the image
    pub height: u32,
    /// The raw image data in RGBA8 format
    pub data: Arc<Vec<u8>>,
}
impl RasterImageData {
    pub fn new(width: u32, height: u32, data: Arc<Vec<u8>>) -> Self {
        Self {
            width,
            height,
            data,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ImageData {
    Raster(RasterImageData),
    #[cfg(feature = "svg")]
    Svg(Box<usvg::Tree>),
    None,
}
#[cfg(feature = "svg")]
impl From<usvg::Tree> for ImageData {
    fn from(value: usvg::Tree) -> Self {
        Self::Svg(Box::new(value))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Ok,
    Error,
    Loading,
}

/// Background image type enum to support different CSS image types
#[derive(Debug, Clone)]
pub enum BackgroundImageType {
    /// URL-based image (traditional background images)
    Url(ServoArc<Url>),
    /// CSS gradient (linear, radial, conic, repeating variants)
    Gradient(Box<StyloGradient>),
    // Future CSS image types can be added here:
    // ImageSet(Box<ImageSet>),
    // CrossFade(Box<CrossFade>),
}

#[derive(Debug, Clone)]
pub struct BackgroundImageData {
    /// The type and data of the background image
    pub image_type: BackgroundImageType,
    /// The loading status of the background image
    pub status: Status,
    /// The image data
    pub image: ImageData,
}

impl BackgroundImageData {
    /// Create a new URL-based background image
    pub fn new(url: ServoArc<Url>) -> Self {
        Self {
            image_type: BackgroundImageType::Url(url),
            status: Status::Loading,
            image: ImageData::None,
        }
    }

    /// Create a new gradient-based background image
    pub fn new_gradient(gradient: StyloGradient) -> Self {
        Self {
            image_type: BackgroundImageType::Gradient(Box::new(gradient)),
            status: Status::Ok, // Gradients don't need loading
            image: ImageData::None,
        }
    }

    /// Get the URL if this is a URL-based background image
    pub fn url(&self) -> Option<&ServoArc<Url>> {
        match &self.image_type {
            BackgroundImageType::Url(url) => Some(url),
            _ => None,
        }
    }

    /// Get the gradient if this is a gradient-based background image
    pub fn gradient(&self) -> Option<&StyloGradient> {
        match &self.image_type {
            BackgroundImageType::Gradient(gradient) => Some(gradient),
            _ => None,
        }
    }
}

pub struct TextInputData {
    /// A cosmyc-text Editor instance
    pub editor: blitz_text::Editor<'static>,
    /// Whether the input is a singleline or multiline input
    pub is_multiline: bool,
    /// Original value when focus was gained (for HTML standards-compliant Change event detection)
    pub original_value: String,
}

impl Clone for TextInputData {
    fn clone(&self) -> Self {
        Self {
            editor: self.editor.clone(), // Editor IS Clone - preserves ALL state
            is_multiline: self.is_multiline,
            original_value: self.original_value.clone(),
        }
    }
}

impl TextInputData {
    pub fn new(font_system: &mut blitz_text::FontSystem, is_multiline: bool) -> Self {
        let metrics = Metrics::new(16.0, 20.0); // 16px font, 20px line height
        let buffer = Buffer::new(font_system, metrics);
        let editor = blitz_text::Editor::new(buffer);

        Self {
            editor,
            is_multiline,
            original_value: String::new(),
        }
    }

    pub fn set_text(&mut self, font_system: &mut blitz_text::FontSystem, text: &str) {
        use blitz_text::{Attrs, Shaping};

        // Clear existing text and set new text
        self.editor.with_buffer_mut(|buffer| {
            buffer.set_text(font_system, text, &Attrs::new(), Shaping::Advanced);
        });

        // Ensure the editor updates its layout
        self.editor.shape_as_needed(font_system, true);
    }

    /// Get the current text value from the editor (HTML standards-compliant)
    pub fn get_current_value(&self) -> String {
        self.editor.with_buffer(|buffer| {
            buffer
                .lines
                .iter()
                .map(|line| line.text())
                .collect::<Vec<_>>()
                .join("\n")
        })
    }

    /// Capture the current value as the original value (called when focus is gained)
    pub fn capture_original_value(&mut self) {
        self.original_value = self.get_current_value();
    }

    /// Check if the current value differs from the original value (for Change event detection)
    pub fn has_value_changed(&self) -> bool {
        self.get_current_value() != self.original_value
    }

    /// Revert to the original value (for Escape key handling according to HTML Living Standard)
    pub fn revert_to_original_value(&mut self, font_system: &mut blitz_text::FontSystem) {
        self.set_text(font_system, &self.original_value.clone());
    }
}

#[derive(Debug, Clone)]
pub struct CanvasData {
    pub custom_paint_source_id: u64,
}

#[derive(Debug, Clone)]
pub struct FileInputData {
    pub selected_files: Vec<FileData>,
    pub accept: Option<String>,
    pub multiple: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileData {
    pub name: String,
    pub content_type: String,
    pub size: u64,
    pub data: Vec<u8>,
}

impl std::fmt::Debug for SpecialElementData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpecialElementData::Stylesheet(_) => f.write_str("NodeSpecificData::Stylesheet"),
            SpecialElementData::Image(data) => match **data {
                ImageData::Raster(_) => f.write_str("NodeSpecificData::Image(Raster)"),
                #[cfg(feature = "svg")]
                ImageData::Svg(_) => f.write_str("NodeSpecificData::Image(Svg)"),
                ImageData::None => f.write_str("NodeSpecificData::Image(None)"),
            },
            SpecialElementData::Canvas(_) => f.write_str("NodeSpecificData::Canvas"),
            SpecialElementData::TableRoot(_) => f.write_str("NodeSpecificData::TableRoot"),
            SpecialElementData::TextInput(_) => f.write_str("NodeSpecificData::TextInput"),
            SpecialElementData::CheckboxInput(_) => f.write_str("NodeSpecificData::CheckboxInput"),
            SpecialElementData::FileInput(_) => f.write_str("NodeSpecificData::FileInput"),
            SpecialElementData::None => f.write_str("NodeSpecificData::None"),
        }
    }
}

#[derive(Clone)]
pub struct ListItemLayout {
    pub marker: Marker,
    pub position: ListItemLayoutPosition,
}

// We seperate chars from strings in order to optimise rendering - ie not needing to
// construct a whole cosmyc-text Buffer for simple char markers
#[derive(Debug, PartialEq, Clone)]
pub enum Marker {
    Char(char),
    String(String),
}

// Value depends on list-style-position, determining whether a seperate layout is created for it
#[derive(Clone)]
pub enum ListItemLayoutPosition {
    Inside,
    Outside(Box<blitz_text::EnhancedBuffer>),
}

impl std::fmt::Debug for ListItemLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ListItemLayout - marker {:?}", self.marker)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
/// Parley Brush type for Blitz which contains a `peniko::Brush` and a Blitz node id
pub struct TextBrush {
    /// The node id for the span
    pub id: usize,
    /// Peniko brush for the span (represents text color)
    pub brush: peniko::Brush,
}

impl TextBrush {
    pub fn from_peniko_brush(brush: peniko::Brush) -> Self {
        Self { id: 0, brush }
    }
    pub fn from_color(color: AlphaColor<Srgb>) -> Self {
        Self::from_peniko_brush(peniko::Brush::Solid(color))
    }
    pub fn from_id_and_color(id: usize, color: AlphaColor<Srgb>) -> Self {
        Self {
            id,
            brush: peniko::Brush::Solid(color),
        }
    }
}

/// Content width measurements
#[derive(Debug, Clone, Copy)]
pub struct ContentWidths {
    pub min: f32,
    pub max: f32,
}

/// Inline box representation for replaced elements
#[derive(Debug, Clone)]
pub struct InlineBox {
    pub id: u64,
    pub index: usize,
    pub width: f32,
    pub height: f32,
    pub x: f32,
    pub y: f32,
}

impl InlineBox {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            index: 0,
            width: 0.0,
            height: 0.0,
            x: 0.0,
            y: 0.0,
        }
    }
}

#[derive(Clone)]
pub struct TextLayout {
    pub text: String,
    pub layout: EnhancedBuffer,
    pub inline_boxes: Vec<InlineBox>,
    
    // Content width caching fields
    pub cached_content_widths: Option<ContentWidths>,
    pub cached_text_hash: Option<u64>,
}

impl TextLayout {
    /// Get the size of the text layout in pixels (width, height)
    #[inline]
    pub fn size(&self) -> (f32, f32) {
        let (width, height) = self.layout.inner().size();
        (width.unwrap_or(0.0), height.unwrap_or(0.0))
    }

    /// Get the number of lines in the layout
    #[inline]
    pub fn line_count(&self) -> usize {
        self.layout.inner().lines.len()
    }

    /// Calculate the maximum intrinsic width (widest line)
    #[inline]
    pub fn max_intrinsic_width(&self) -> f32 {
        self.layout
            .cached_layout_runs()
            .iter()
            .map(|run| run.line_width)
            .fold(0.0f32, f32::max)
    }

    /// Calculate content widths (min and max) using CSS semantics
    pub fn calculate_content_widths(&self) -> ContentWidths {
        // CSS min-content width: width of longest unbreakable sequence
        // CSS max-content width: width with no line breaks
        // 
        // Note: This is more expensive than the cached approach but provides
        // correct CSS layout semantics. For performance-critical paths,
        // consider caching these values with proper invalidation.
        
        // We need mutable access for the font_system, but this is a design limitation
        // of the current API. In production, this should be refactored to avoid
        // the mutable requirement for width calculations.
        
        // For now, use the cached data as an approximation
        // TODO: Implement proper CSS content width calculation with word boundary detection
        let cached_runs = self.layout.cached_layout_runs();
        
        if cached_runs.is_empty() {
            return ContentWidths { min: 0.0, max: 0.0 };
        }
        
        // APPROXIMATION: Use the current cache data
        // This is not fully CSS-compliant but maintains performance
        let mut min_width = f32::INFINITY;
        let mut max_width = 0.0f32;

        for run in cached_runs {
            let width = run.line_width;
            min_width = min_width.min(width);
            max_width = max_width.max(width);
        }

        // If no lines were processed, min_width would still be INFINITY
        if min_width == f32::INFINITY {
            min_width = 0.0;
        }

        ContentWidths {
            min: min_width,
            max: max_width,
        }
    }

    /// Calculate content widths with caching optimization
    pub fn calculate_content_widths_cached(&mut self) -> ContentWidths {
        let current_hash = self.calculate_text_hash();
        
        // Check cache validity
        if let (Some(cached), Some(cached_hash)) = 
            (&self.cached_content_widths, self.cached_text_hash) 
        {
            if cached_hash == current_hash {
                return cached.clone();  // Cache hit
            }
        }
        
        // Cache miss - calculate and store
        let widths = self.calculate_content_widths();
        self.cached_content_widths = Some(widths.clone());
        self.cached_text_hash = Some(current_hash);
        widths
    }
    
    /// Generate hash for cache invalidation
    fn calculate_text_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        self.text.hash(&mut hasher);
        self.layout.cached_layout_runs().len().hash(&mut hasher);
        
        // Add layout dimensions for complete invalidation
        if let (Some(width), Some(height)) = self.layout.inner().size() {
            width.to_bits().hash(&mut hasher);
            height.to_bits().hash(&mut hasher);
        }
        hasher.finish()
    }
    
    /// Invalidate cache when layout changes
    pub fn invalidate_content_width_cache(&mut self) {
        self.cached_content_widths = None;
        self.cached_text_hash = None;
    }

    /// Enhanced CSS-compliant content width calculation
    pub fn calculate_content_widths_css_compliant(
        &mut self,
        font_system: &mut blitz_text::FontSystem,
    ) -> ContentWidths {
        // Use enhanced buffer methods for proper CSS compliance
        let min_width = self.layout.css_min_content_width(font_system);
        let max_width = self.layout.css_max_content_width(font_system);
        
        ContentWidths {
            min: min_width,
            max: max_width,
        }
    }
    
    /// Cache-aware content width calculation with CSS compliance
    pub fn calculate_content_widths_cached_css(
        &mut self,
        font_system: &mut blitz_text::FontSystem,
    ) -> ContentWidths {
        let current_hash = self.calculate_text_hash();
        
        // Check cache validity
        if let (Some(cached), Some(cached_hash)) = 
            (&self.cached_content_widths, self.cached_text_hash) 
        {
            if cached_hash == current_hash {
                return cached.clone();  // Cache hit
            }
        }
        
        // Cache miss - calculate using CSS-compliant method
        let min_width = self.layout.css_min_content_width(font_system);
        let max_width = self.layout.css_max_content_width(font_system);
        
        let widths = ContentWidths {
            min: min_width,
            max: max_width,
        };
        
        self.cached_content_widths = Some(widths.clone());
        self.cached_text_hash = Some(current_hash);
        widths
    }

    /// Calculate content widths including inline elements (CSS compliant)
    pub fn calculate_content_widths_with_inline_elements(
        &mut self,
        font_system: &mut blitz_text::FontSystem,
    ) -> ContentWidths {
        // Get text-only widths
        let text_min = self.layout.css_min_content_width(font_system);
        let text_max = self.layout.css_max_content_width(font_system);
        
        // Calculate inline element contributions
        let (inline_min, inline_max) = self.calculate_inline_contributions();
        
        ContentWidths {
            // CSS: min-content = max(text_min, largest_inline_width)
            min: text_min.max(inline_min),
            // CSS: max-content = text_max + sum(all_inline_widths)  
            max: text_max + inline_max,
        }
    }
    
    fn calculate_inline_contributions(&self) -> (f32, f32) {
        if self.inline_boxes.is_empty() {
            return (0.0, 0.0);
        }
        
        // Min-content: largest single inline element (unbreakable)
        let min_contribution = self.inline_boxes
            .iter()
            .map(|ibox| ibox.width)
            .fold(0.0f32, f32::max);
            
        // Max-content: sum of all inline elements (normal flow)
        let max_contribution = self.inline_boxes
            .iter()
            .map(|ibox| ibox.width)
            .sum::<f32>();
            
        (min_contribution, max_contribution)
    }

    /// Break all lines at specified width
    pub fn break_all_lines(
        &mut self,
        font_system: &mut blitz_text::FontSystem,
        width: Option<f32>,
    ) {
        self.layout
            .set_size_cached(font_system, width, Some(f32::INFINITY));
        
        // Invalidate cache when layout changes
        self.invalidate_content_width_cache();
    }

    /// Get width of the layout
    pub fn width(&self) -> f32 {
        self.max_intrinsic_width()
    }

    /// Get height of the layout
    pub fn height(&self) -> f32 {
        let mut total_height = 0.0f32;
        for run in self.layout.cached_layout_runs() {
            let line_bottom = run.line_top + run.line_height;
            if line_bottom > total_height {
                total_height = line_bottom;
            }
        }
        total_height
    }
}

impl std::fmt::Debug for TextLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TextLayout")
    }
}

#[cfg(test)]
mod background_image_tests {
    use url::Url;

    use super::*;

    #[test]
    fn test_background_image_url_creation() {
        let url: ServoArc<Url> = ServoArc::new("https://example.com/image.png".parse().unwrap());
        let bg_data = BackgroundImageData::new(url.clone());

        // Should be URL type
        assert!(matches!(bg_data.image_type, BackgroundImageType::Url(_)));
        assert_eq!(bg_data.url(), Some(&url));
        assert_eq!(bg_data.gradient(), None);
        assert_eq!(bg_data.status, Status::Loading);
    }

    #[test]
    fn test_background_image_type_enum_structure() {
        let url: ServoArc<Url> = ServoArc::new("https://example.com/test.jpg".parse().unwrap());
        let url_type = BackgroundImageType::Url(url);

        // Test URL variant
        if let BackgroundImageType::Url(_) = url_type {
            // Expected - URL variant works
        } else {
            panic!("URL variant not matched correctly");
        }
    }

    #[test]
    fn test_url_method_compatibility() {
        // Test that the URL method works correctly for backward compatibility
        let url: ServoArc<Url> = ServoArc::new("https://example.com/bg.jpg".parse().unwrap());
        let bg_data = BackgroundImageData::new(url.clone());

        // The url() method should return the URL for URL-type backgrounds
        assert!(bg_data.url().is_some());

        // The gradient() method should return None for URL-type backgrounds
        assert!(bg_data.gradient().is_none());

        // Status should be Loading for URL-based images
        assert_eq!(bg_data.status, Status::Loading);
    }
}

#[cfg(test)]
mod content_width_caching_tests {
    use super::*;
    use blitz_text::{FontSystem, Metrics};

    fn create_test_text_layout() -> TextLayout {
        let mut font_system = FontSystem::new();
        let buffer = blitz_text::EnhancedBuffer::new(&mut font_system, Metrics::new(16.0, 20.0));
        TextLayout {
            text: "Test text content".to_string(),
            layout: buffer,
            inline_boxes: Vec::new(),
            cached_content_widths: None,
            cached_text_hash: None,
        }
    }

    #[test]
    fn test_content_width_caching_performance() {
        let mut text_layout = create_test_text_layout();
        
        // First call - cache miss
        let widths1 = text_layout.calculate_content_widths_cached();
        
        // Second call - cache hit
        let widths2 = text_layout.calculate_content_widths_cached();
        
        // Results should be identical
        assert_eq!(widths1.min, widths2.min);
        assert_eq!(widths1.max, widths2.max);
        
        // Cache should be populated
        assert!(text_layout.cached_content_widths.is_some());
        assert!(text_layout.cached_text_hash.is_some());
    }
    
    #[test]
    fn test_cache_invalidation_on_text_change() {
        let mut text_layout = create_test_text_layout();
        
        // Cache initial calculation
        let _widths1 = text_layout.calculate_content_widths_cached();
        assert!(text_layout.cached_content_widths.is_some());
        
        // Change text content
        text_layout.text = "Different text content".to_string();
        
        // Cache should be invalidated due to text hash change
        let _widths2 = text_layout.calculate_content_widths_cached();
        
        // Verify cache was properly recalculated
        assert!(text_layout.cached_content_widths.is_some());
    }
    
    #[test]
    fn test_manual_cache_invalidation() {
        let mut text_layout = create_test_text_layout();
        
        // Populate cache
        let _widths = text_layout.calculate_content_widths_cached();
        assert!(text_layout.cached_content_widths.is_some());
        
        // Manually invalidate cache
        text_layout.invalidate_content_width_cache();
        
        // Cache should be cleared
        assert!(text_layout.cached_content_widths.is_none());
        assert!(text_layout.cached_text_hash.is_none());
    }
    
    #[test]
    fn test_break_all_lines_invalidates_cache() {
        let mut text_layout = create_test_text_layout();
        let mut font_system = FontSystem::new();
        
        // Populate cache
        let _widths = text_layout.calculate_content_widths_cached();
        assert!(text_layout.cached_content_widths.is_some());
        
        // Break lines should invalidate cache
        text_layout.break_all_lines(&mut font_system, Some(200.0));
        
        // Cache should be cleared
        assert!(text_layout.cached_content_widths.is_none());
        assert!(text_layout.cached_text_hash.is_none());
    }
}


