use accesskit_winit::Adapter;
use blitz_dom::BaseDocument;
use winit::{
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::Window,
};

use crate::event::BlitzShellEvent;

/// State of the accessibility node tree and platform adapter.
pub struct AccessibilityState {
    /// Adapter to connect to the [`EventLoop`](`winit::event_loop::EventLoop`).
    adapter: accesskit_winit::Adapter,
}

impl AccessibilityState {
    pub fn new(
        event_loop: &ActiveEventLoop,
        window: &Window,
        proxy: EventLoopProxy<BlitzShellEvent>,
    ) -> Self {
        Self {
            adapter: Adapter::with_event_loop_proxy(event_loop, window, proxy),
        }
    }
    pub fn update_tree(&mut self, doc: &BaseDocument) {
        self.adapter
            .update_if_active(|| doc.build_accessibility_tree());
    }
}
