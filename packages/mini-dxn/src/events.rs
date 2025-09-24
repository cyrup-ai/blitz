use std::any::Any;
use std::collections::HashMap;

use blitz_traits::events::{BlitzKeyEvent, BlitzMouseButtonEvent, MouseEventButton, MouseEventButtons};
use dioxus_html::{
    AnimationData, ClipboardData, CompositionData, DragData, FocusData, FormData, FormValue,
    HasAnimationData, HasClipboardData, HasCompositionData, HasDragData, HasFileData, HasFocusData, 
    HasFormData, HasImageData, HasKeyboardData, HasMediaData, HasMouseData,
    HasPointerData, HasResizeData, HasScrollData, HasSelectionData, HasToggleData, HasTouchData,
    HasTouchPointData, HasTransitionData, HasVisibleData, HasWheelData, HtmlEventConverter, ImageData, KeyboardData, 
    MediaData, MountedData, MouseData, PlatformEventData, PointerData, ResizeData, ScrollData, 
    SelectionData, ToggleData, TouchData, TransitionData, VisibleData, WheelData,
    geometry::{ClientPoint, ElementPoint, PagePoint, ScreenPoint},
    input_data::{MouseButton, MouseButtonSet},
    geometry::WheelDelta,
    TouchPoint,
    point_interaction::{
        InteractionElementOffset, InteractionLocation, ModifiersInteraction, PointerInteraction,
    },
};
use keyboard_types::{Code, Key, Location, Modifiers};

#[derive(Clone, Debug)]
pub struct NativeClickData {
    /// Client coordinates (relative to viewport)
    pub client_point: ClientPoint,
    /// Screen coordinates (relative to screen) 
    pub screen_point: ScreenPoint,
    /// Page coordinates (relative to document)
    pub page_point: PagePoint,
    /// Element coordinates (relative to target element)
    pub element_point: ElementPoint,
    /// Keyboard modifiers active when event occurred
    pub modifiers: Modifiers,
    /// Mouse button that triggered the event
    pub trigger_button: Option<MouseButton>,
    /// Set of mouse buttons held when event occurred
    pub held_buttons: MouseButtonSet,
}

impl InteractionLocation for NativeClickData {
    fn client_coordinates(&self) -> ClientPoint {
        self.client_point
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        self.screen_point
    }

    fn page_coordinates(&self) -> PagePoint {
        self.page_point
    }
}

impl InteractionElementOffset for NativeClickData {
    fn element_coordinates(&self) -> ElementPoint {
        self.element_point
    }
}

impl ModifiersInteraction for NativeClickData {
    fn modifiers(&self) -> Modifiers {
        self.modifiers
    }
}

impl PointerInteraction for NativeClickData {
    fn trigger_button(&self) -> Option<MouseButton> {
        self.trigger_button
    }

    fn held_buttons(&self) -> MouseButtonSet {
        self.held_buttons
    }
}

impl NativeClickData {
    /// Create a new NativeClickData from a BlitzMouseButtonEvent
    /// 
    /// This converts between the blitz coordinate system (f32) and dioxus coordinate system (f64),
    /// and between blitz mouse button types and dioxus mouse button types.
    pub fn new(event: BlitzMouseButtonEvent) -> Self {
        // Convert coordinates from f32 to f64
        let x = event.x as f64;
        let y = event.y as f64;
        
        // For now, treat the blitz coordinates as client coordinates (relative to viewport)
        // In a complete implementation, these would be derived from window/element positions
        let client_point = ClientPoint::new(x, y);
        
        // Screen coordinates would typically add window position offset
        // For now, assume no window offset (fullscreen or positioned at 0,0)
        let screen_point = ScreenPoint::new(x, y);
        
        // Page coordinates would add scroll offset
        // For now, assume no scrolling
        let page_point = PagePoint::new(x, y);
        
        // Element coordinates would subtract element position
        // For now, assume click is at element origin
        let element_point = ElementPoint::new(0.0, 0.0);
        
        // Convert button from blitz MouseEventButton to dioxus MouseButton
        let trigger_button = Self::convert_mouse_button(event.button);
        
        // Convert button set from blitz MouseEventButtons to dioxus MouseButtonSet
        let held_buttons = Self::convert_mouse_button_set(event.buttons);
        
        Self {
            client_point,
            screen_point,
            page_point,
            element_point,
            modifiers: event.mods,
            trigger_button,
            held_buttons,
        }
    }
    
    /// Convert blitz MouseEventButton to dioxus MouseButton
    fn convert_mouse_button(button: MouseEventButton) -> Option<MouseButton> {
        match button {
            MouseEventButton::Main => Some(MouseButton::Primary),
            MouseEventButton::Auxiliary => Some(MouseButton::Auxiliary),
            MouseEventButton::Secondary => Some(MouseButton::Secondary),
            MouseEventButton::Fourth => Some(MouseButton::Fourth),
            MouseEventButton::Fifth => Some(MouseButton::Fifth),
        }
    }
    
    /// Convert blitz MouseEventButtons to dioxus MouseButtonSet
    fn convert_mouse_button_set(buttons: MouseEventButtons) -> MouseButtonSet {
        let mut set = MouseButtonSet::empty();
        
        if buttons.contains(MouseEventButtons::Primary) {
            set |= MouseButton::Primary;
        }
        if buttons.contains(MouseEventButtons::Secondary) {
            set |= MouseButton::Secondary;
        }
        if buttons.contains(MouseEventButtons::Auxiliary) {
            set |= MouseButton::Auxiliary;
        }
        if buttons.contains(MouseEventButtons::Fourth) {
            set |= MouseButton::Fourth;
        }
        if buttons.contains(MouseEventButtons::Fifth) {
            set |= MouseButton::Fifth;
        }
        
        set
    }
}

impl HasMouseData for NativeClickData {
    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }
}

impl Default for NativeClickData {
    fn default() -> Self {
        Self {
            client_point: ClientPoint::new(0.0, 0.0),
            screen_point: ScreenPoint::new(0.0, 0.0),
            page_point: PagePoint::new(0.0, 0.0),
            element_point: ElementPoint::new(0.0, 0.0),
            modifiers: Modifiers::empty(),
            trigger_button: None,
            held_buttons: MouseButtonSet::empty(),
        }
    }
}

pub struct NativeConverter {}

impl HtmlEventConverter for NativeConverter {
    fn convert_animation_data(&self, _event: &PlatformEventData) -> AnimationData {
        // Animation events don't carry specific data in this implementation
        // Create with default native animation data structure
        AnimationData::new(NativeAnimationData::default())
    }

    fn convert_clipboard_data(&self, _event: &PlatformEventData) -> ClipboardData {
        // Clipboard operations are handled by the shell provider
        // Create with default native clipboard data structure
        ClipboardData::new(NativeClipboardData::default())
    }

    fn convert_composition_data(&self, event: &PlatformEventData) -> CompositionData {
        let data = match event.downcast::<NativeCompositionData>() {
            Some(data) => data.clone(),
            None => NativeCompositionData {
                data: String::new(),
            },
        };
        CompositionData::from(data)
    }

    fn convert_drag_data(&self, _event: &PlatformEventData) -> DragData {
        // Drag events not implemented yet - create with default data
        DragData::new(NativeDragData::default())
    }

    fn convert_focus_data(&self, event: &PlatformEventData) -> FocusData {
        let data = match event.downcast::<NativeFocusData>() {
            Some(data) => data.clone(),
            None => NativeFocusData {},
        };
        FocusData::new(data)
    }

    fn convert_form_data(&self, event: &PlatformEventData) -> FormData {
        let o = event.downcast::<NativeFormData>().unwrap().clone();
        FormData::from(o)
    }

    fn convert_image_data(&self, _event: &PlatformEventData) -> ImageData {
        // Image load/error events - create with default data
        ImageData::new(NativeImageData::default())
    }

    fn convert_keyboard_data(&self, event: &PlatformEventData) -> KeyboardData {
        let data = event.downcast::<BlitzKeyboardData>().unwrap().clone();
        KeyboardData::from(data)
    }

    fn convert_media_data(&self, _event: &PlatformEventData) -> MediaData {
        // Audio/video media events - create with default data
        MediaData::new(NativeMediaData::default())
    }

    fn convert_mounted_data(&self, _event: &PlatformEventData) -> MountedData {
        // Component mount events - create with unit type that implements RenderedElementBacking
        MountedData::new(())
    }

    fn convert_mouse_data(&self, event: &PlatformEventData) -> MouseData {
        let o = event.downcast::<NativeClickData>().unwrap().clone();
        MouseData::from(o)
    }

    fn convert_pointer_data(&self, event: &PlatformEventData) -> PointerData {
        // Convert mouse data to pointer data if available
        if let Some(mouse_data) = event.downcast::<NativeClickData>() {
            PointerData::new(NativePointerData::from_mouse_data(mouse_data.clone()))
        } else {
            PointerData::new(NativePointerData::default())
        }
    }

    fn convert_scroll_data(&self, _event: &PlatformEventData) -> ScrollData {
        // Scroll events - create with default data
        ScrollData::new(NativeScrollData::default())
    }

    fn convert_selection_data(&self, _event: &PlatformEventData) -> SelectionData {
        // Text selection events - create with default data
        SelectionData::new(NativeSelectionData::default())
    }

    fn convert_toggle_data(&self, _event: &PlatformEventData) -> ToggleData {
        // Checkbox/radio button toggle events - create with default data
        ToggleData::new(NativeToggleData::default())
    }

    fn convert_touch_data(&self, _event: &PlatformEventData) -> TouchData {
        // Touch events not implemented yet - create with default data
        TouchData::new(NativeTouchData::default())
    }

    fn convert_transition_data(&self, _event: &PlatformEventData) -> TransitionData {
        // CSS transition events - create with default data
        TransitionData::new(NativeTransitionData::default())
    }

    fn convert_wheel_data(&self, _event: &PlatformEventData) -> WheelData {
        // Mouse wheel events - create with default data
        WheelData::new(NativeWheelData::default())
    }

    fn convert_resize_data(&self, _event: &PlatformEventData) -> ResizeData {
        // Element resize events - create with default data
        ResizeData::new(NativeResizeData::default())
    }

    fn convert_visible_data(&self, _event: &PlatformEventData) -> VisibleData {
        // Intersection observer events - create with default data
        VisibleData::new(NativeVisibleData::default())
    }
}

// Native event data structures

#[derive(Clone, Debug, Default)]
pub struct NativeAnimationData {
    pub animation_name: String,
    pub elapsed_time: f32,
    pub pseudo_element: String,
}

#[derive(Clone, Debug, Default)]
pub struct NativeClipboardData {
    // Clipboard operations are handled by shell provider
    // This is just a placeholder for the trait system
}

#[derive(Clone, Debug, Default)]
pub struct NativeDragData {
    #[allow(dead_code)] // Infrastructure for future drag data transfer implementation
    pub data_transfer: HashMap<String, String>,
    #[allow(dead_code)] // Infrastructure for future drag effect handling
    pub effect_allowed: String,
    #[allow(dead_code)] // Infrastructure for future drag drop effect handling
    pub drop_effect: String,
    pub mouse_data: NativeClickData,
}

#[derive(Clone, Debug, Default)]
pub struct NativeImageData {
    #[allow(dead_code)] // Infrastructure for image natural dimensions
    pub natural_width: u32,
    #[allow(dead_code)] // Infrastructure for image natural dimensions
    pub natural_height: u32,
    pub complete: bool,
}

#[derive(Clone, Debug, Default)]  
pub struct NativeMediaData {
    #[allow(dead_code)] // Infrastructure for media playback time tracking
    pub current_time: f64,
    #[allow(dead_code)] // Infrastructure for media duration tracking
    pub duration: f64,
    #[allow(dead_code)] // Infrastructure for media volume control
    pub volume: f64,
    #[allow(dead_code)] // Infrastructure for media mute state
    pub muted: bool,
    #[allow(dead_code)] // Infrastructure for media play/pause state
    pub paused: bool,
}

#[derive(Clone, Debug, Default)]
#[allow(dead_code)] // Infrastructure for component mount events
pub struct NativeMountedData {
    // Component mount data - placeholder for now
}

#[derive(Clone, Debug)]
pub struct NativePointerData {
    pub pointer_id: i32,
    pub width: f64,
    pub height: f64,
    pub pressure: f64,
    pub tangential_pressure: f64,
    pub tilt_x: f64,
    pub tilt_y: f64,
    pub twist: f64,
    pub pointer_type: String,
    pub is_primary: bool,
    pub mouse_data: NativeClickData,
}

impl NativePointerData {
    pub fn from_mouse_data(mouse_data: NativeClickData) -> Self {
        Self {
            pointer_id: 1, // Mouse is always pointer ID 1
            width: 1.0,    // Default mouse pointer size
            height: 1.0,
            pressure: if mouse_data.held_buttons.is_empty() { 0.0 } else { 0.5 },
            tangential_pressure: 0.0,
            tilt_x: 0.0,
            tilt_y: 0.0,
            twist: 0.0,
            pointer_type: "mouse".to_string(),
            is_primary: true,
            mouse_data,
        }
    }
}

impl Default for NativePointerData {
    fn default() -> Self {
        Self::from_mouse_data(NativeClickData::default())
    }
}

#[derive(Clone, Debug, Default)]
pub struct NativeScrollData {
    #[allow(dead_code)] // Infrastructure for scroll delta X
    pub delta_x: f64,
    #[allow(dead_code)] // Infrastructure for scroll delta Y
    pub delta_y: f64,
    #[allow(dead_code)] // Infrastructure for scroll delta Z
    pub delta_z: f64,
    #[allow(dead_code)] // Infrastructure for scroll delta mode
    pub delta_mode: u32,
}

#[derive(Clone, Debug, Default)]
pub struct NativeSelectionData {
    #[allow(dead_code)] // Infrastructure for text selection start position
    pub selection_start: Option<u32>,
    #[allow(dead_code)] // Infrastructure for text selection end position
    pub selection_end: Option<u32>,
    #[allow(dead_code)] // Infrastructure for text selection direction
    pub selection_direction: String,
}

#[derive(Clone, Debug, Default)]
pub struct NativeToggleData {
    #[allow(dead_code)] // Infrastructure for checkbox/radio button state
    pub checked: bool,
}

#[derive(Clone, Debug, Default)]
pub struct NativeTouchData {
    pub touches: Vec<NativeTouchPoint>,
    pub target_touches: Vec<NativeTouchPoint>,
    pub changed_touches: Vec<NativeTouchPoint>,
}

#[derive(Clone, Debug, Default)]
pub struct NativeTouchPoint {
    pub identifier: i32,
    pub screen_x: f64,
    pub screen_y: f64,
    pub client_x: f64,
    pub client_y: f64,
    pub page_x: f64,
    pub page_y: f64,
    pub radius_x: f64,
    pub radius_y: f64,
    pub rotation_angle: f64,
    pub force: f64,
}

impl InteractionLocation for NativeTouchPoint {
    fn client_coordinates(&self) -> ClientPoint {
        ClientPoint::new(self.client_x, self.client_y)
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        ScreenPoint::new(self.screen_x, self.screen_y)
    }

    fn page_coordinates(&self) -> PagePoint {
        PagePoint::new(self.page_x, self.page_y)
    }
}

impl HasTouchPointData for NativeTouchPoint {
    fn identifier(&self) -> i32 {
        self.identifier
    }

    fn force(&self) -> f64 {
        self.force
    }

    fn radius(&self) -> ScreenPoint {
        ScreenPoint::new(self.radius_x, self.radius_y)
    }

    fn rotation(&self) -> f64 {
        self.rotation_angle
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Clone, Debug, Default)]
pub struct NativeTransitionData {
    pub property_name: String,
    pub elapsed_time: f32,
    pub pseudo_element: String,
}

#[derive(Clone, Debug)]
pub struct NativeWheelData {
    pub delta_x: f64,
    pub delta_y: f64,
    pub delta_z: f64,
    pub delta_mode: u32, // 0 = pixels, 1 = lines, 2 = pages
    pub mouse_data: NativeClickData,
}

impl Default for NativeWheelData {
    fn default() -> Self {
        Self {
            delta_x: 0.0,
            delta_y: 0.0,
            delta_z: 0.0,
            delta_mode: 0,
            mouse_data: NativeClickData::default(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct NativeResizeData {
    #[allow(dead_code)] // Infrastructure for element resize width
    pub width: f64,
    #[allow(dead_code)] // Infrastructure for element resize height
    pub height: f64,
}

#[derive(Clone, Debug, Default)]
pub struct NativeVisibleData {
    #[allow(dead_code)] // Infrastructure for intersection observer ratio
    pub intersection_ratio: f64,
    #[allow(dead_code)] // Infrastructure for intersection observer state
    pub is_intersecting: bool,
}

#[derive(Clone, Debug)]
pub struct NativeFormData {
    pub value: String,
    pub values: HashMap<String, FormValue>,
}

impl HasFormData for NativeFormData {
    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }

    fn value(&self) -> String {
        self.value.clone()
    }

    fn values(&self) -> HashMap<String, FormValue> {
        self.values.clone()
    }
}

impl HasFileData for NativeFormData {}

#[derive(Clone, Debug)]
pub(crate) struct BlitzKeyboardData(pub(crate) BlitzKeyEvent);

impl ModifiersInteraction for BlitzKeyboardData {
    fn modifiers(&self) -> Modifiers {
        self.0.modifiers
    }
}

impl HasKeyboardData for BlitzKeyboardData {
    fn key(&self) -> Key {
        self.0.key.clone()
    }

    fn code(&self) -> Code {
        self.0.code
    }

    fn location(&self) -> Location {
        self.0.location
    }

    fn is_auto_repeating(&self) -> bool {
        self.0.is_auto_repeating
    }

    fn is_composing(&self) -> bool {
        self.0.is_composing
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn Any
    }
}

#[derive(Clone, Debug)]
pub(crate) struct NativeCompositionData {
    pub(crate) data: String,
}

#[derive(Clone, Debug)]
pub(crate) struct NativeFocusData {}

impl HasFocusData for NativeFocusData {
    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn Any
    }
}

impl HasCompositionData for NativeCompositionData {
    fn data(&self) -> String {
        self.data.clone()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn Any
    }
}

// Trait implementations for all native event data structures

impl HasAnimationData for NativeAnimationData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn animation_name(&self) -> String {
        self.animation_name.clone()
    }

    fn pseudo_element(&self) -> String {
        self.pseudo_element.clone()
    }

    fn elapsed_time(&self) -> f32 {
        self.elapsed_time
    }
}

impl HasClipboardData for NativeClipboardData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasDragData for NativeDragData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasMouseData for NativeDragData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasFileData for NativeDragData {}

impl InteractionLocation for NativeDragData {
    fn client_coordinates(&self) -> ClientPoint {
        self.mouse_data.client_coordinates()
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        self.mouse_data.screen_coordinates()
    }

    fn page_coordinates(&self) -> PagePoint {
        self.mouse_data.page_coordinates()
    }
}

impl InteractionElementOffset for NativeDragData {
    fn element_coordinates(&self) -> ElementPoint {
        self.mouse_data.element_coordinates()
    }
}

impl ModifiersInteraction for NativeDragData {
    fn modifiers(&self) -> Modifiers {
        self.mouse_data.modifiers()
    }
}

impl PointerInteraction for NativeDragData {
    fn trigger_button(&self) -> Option<MouseButton> {
        self.mouse_data.trigger_button()
    }

    fn held_buttons(&self) -> MouseButtonSet {
        self.mouse_data.held_buttons()
    }
}

impl HasImageData for NativeImageData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn load_error(&self) -> bool {
        !self.complete
    }
}

impl HasMediaData for NativeMediaData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Note: HasMountedData trait doesn't exist in current dioxus version
// Mounted data support would be added when the trait becomes available

impl HasPointerData for NativePointerData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn pointer_id(&self) -> i32 {
        self.pointer_id
    }

    fn width(&self) -> i32 {
        self.width as i32
    }

    fn height(&self) -> i32 {
        self.height as i32
    }

    fn pressure(&self) -> f32 {
        self.pressure as f32
    }

    fn tangential_pressure(&self) -> f32 {
        self.tangential_pressure as f32
    }

    fn tilt_x(&self) -> i32 {
        self.tilt_x as i32
    }

    fn tilt_y(&self) -> i32 {
        self.tilt_y as i32
    }

    fn twist(&self) -> i32 {
        self.twist as i32
    }

    fn pointer_type(&self) -> String {
        self.pointer_type.clone()
    }

    fn is_primary(&self) -> bool {
        self.is_primary
    }
}

impl InteractionLocation for NativePointerData {
    fn client_coordinates(&self) -> ClientPoint {
        self.mouse_data.client_coordinates()
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        self.mouse_data.screen_coordinates()
    }

    fn page_coordinates(&self) -> PagePoint {
        self.mouse_data.page_coordinates()
    }
}

impl InteractionElementOffset for NativePointerData {
    fn element_coordinates(&self) -> ElementPoint {
        self.mouse_data.element_coordinates()
    }
}

impl ModifiersInteraction for NativePointerData {
    fn modifiers(&self) -> Modifiers {
        self.mouse_data.modifiers()
    }
}

impl PointerInteraction for NativePointerData {
    fn trigger_button(&self) -> Option<MouseButton> {
        self.mouse_data.trigger_button()
    }

    fn held_buttons(&self) -> MouseButtonSet {
        self.mouse_data.held_buttons()
    }
}

impl HasScrollData for NativeScrollData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn scroll_top(&self) -> i32 {
        0 // Default scroll position
    }

    fn scroll_left(&self) -> i32 {
        0 // Default scroll position
    }

    fn scroll_width(&self) -> i32 {
        1000 // Default scroll area width
    }

    fn scroll_height(&self) -> i32 {
        1000 // Default scroll area height
    }

    fn client_width(&self) -> i32 {
        800 // Default client area width
    }

    fn client_height(&self) -> i32 {
        600 // Default client area height
    }
}

impl HasSelectionData for NativeSelectionData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasToggleData for NativeToggleData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasTouchData for NativeTouchData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn touches(&self) -> Vec<TouchPoint> {
        self.touches.iter().map(|touch| TouchPoint::new(touch.clone())).collect()
    }

    fn touches_changed(&self) -> Vec<TouchPoint> {
        self.changed_touches.iter().map(|touch| TouchPoint::new(touch.clone())).collect()
    }

    fn target_touches(&self) -> Vec<TouchPoint> {
        self.target_touches.iter().map(|touch| TouchPoint::new(touch.clone())).collect()
    }
}

impl ModifiersInteraction for NativeTouchData {
    fn modifiers(&self) -> Modifiers {
        Modifiers::empty() // Default to no modifiers for touch events
    }
}

impl HasTransitionData for NativeTransitionData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn property_name(&self) -> String {
        self.property_name.clone()
    }

    fn pseudo_element(&self) -> String {
        self.pseudo_element.clone()
    }

    fn elapsed_time(&self) -> f32 {
        self.elapsed_time
    }
}

impl HasWheelData for NativeWheelData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn delta(&self) -> WheelDelta {
        WheelDelta::from_web_attributes(self.delta_mode, self.delta_x, self.delta_y, self.delta_z)
    }
}

impl HasMouseData for NativeWheelData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl InteractionLocation for NativeWheelData {
    fn client_coordinates(&self) -> ClientPoint {
        self.mouse_data.client_coordinates()
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        self.mouse_data.screen_coordinates()
    }

    fn page_coordinates(&self) -> PagePoint {
        self.mouse_data.page_coordinates()
    }
}

impl InteractionElementOffset for NativeWheelData {
    fn element_coordinates(&self) -> ElementPoint {
        self.mouse_data.element_coordinates()
    }
}

impl ModifiersInteraction for NativeWheelData {
    fn modifiers(&self) -> Modifiers {
        self.mouse_data.modifiers()
    }
}

impl PointerInteraction for NativeWheelData {
    fn trigger_button(&self) -> Option<MouseButton> {
        self.mouse_data.trigger_button()
    }

    fn held_buttons(&self) -> MouseButtonSet {
        self.mouse_data.held_buttons()
    }
}

impl HasResizeData for NativeResizeData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasVisibleData for NativeVisibleData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

