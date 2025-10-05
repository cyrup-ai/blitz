use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use anyrender::WindowRenderer;
#[cfg(feature = "gpu_backend")]
pub use anyrender_vello::{
    CustomPaintSource, VelloWindowRenderer as InnerRenderer,
    wgpu::{Features, Limits},
};
#[cfg(all(feature = "cpu_backend", not(feature = "gpu_backend")))]
use anyrender_vello_cpu::VelloCpuWindowRenderer as InnerRenderer;

#[cfg(feature = "gpu_backend")]
pub fn use_wgpu<T: CustomPaintSource>(create_source: impl FnOnce() -> T) -> u64 {
    use dioxus_core::{consume_context, use_hook};

    // Register paint source on first render only, keep it alive for the lifetime of the app
    // Don't use cleanup hooks since VirtualDom mutations can trigger premature cleanup
    use_hook(|| {
        let renderer = consume_context::<DxnWindowRenderer>();
        let source = Box::new(create_source());
        let id = renderer.register_custom_paint_source(source);
        id
    })
}

#[derive(Clone)]
pub struct DxnWindowRenderer {
    inner: Rc<RefCell<InnerRenderer>>,
}

impl Default for DxnWindowRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl DxnWindowRenderer {
    pub fn new() -> Self {
        let vello_renderer = InnerRenderer::new();
        let result = Self::with_inner_renderer(vello_renderer);
        println!("🟡 DxnWindowRenderer::new() - created with Rc ptr: {:p}", Rc::as_ptr(&result.inner));
        result
    }

    #[cfg(feature = "gpu_backend")]
    pub fn with_features_and_limits(features: Option<Features>, limits: Option<Limits>) -> Self {
        let vello_renderer = InnerRenderer::with_features_and_limits(features, limits);
        let result = Self::with_inner_renderer(vello_renderer);
        println!("🟡 DxnWindowRenderer::with_features_and_limits() - created with Rc ptr: {:p}", Rc::as_ptr(&result.inner));
        result
    }

    fn with_inner_renderer(vello_renderer: InnerRenderer) -> Self {
        Self {
            inner: Rc::new(RefCell::new(vello_renderer)),
        }
    }
}

impl DxnWindowRenderer {
    #[cfg(feature = "gpu_backend")]
    pub fn register_custom_paint_source(&self, source: Box<dyn CustomPaintSource>) -> u64 {
        let ptr = Rc::as_ptr(&self.inner);
        println!("🔵 DxnWindowRenderer::register_custom_paint_source - Rc ptr: {:p}", ptr);
        self.inner.borrow_mut().register_custom_paint_source(source)
    }

    #[cfg(feature = "gpu_backend")]
    pub fn unregister_custom_paint_source(&self, id: u64) {
        self.inner.borrow_mut().unregister_custom_paint_source(id)
    }


}

impl WindowRenderer for DxnWindowRenderer {
    type ScenePainter<'a>
        = <InnerRenderer as WindowRenderer>::ScenePainter<'a>
    where
        Self: 'a;

    fn resume(&mut self, window: Arc<dyn anyrender::WindowHandle>, width: u32, height: u32) {
        self.inner.borrow_mut().resume(window, width, height)
    }

    fn suspend(&mut self) {
        self.inner.borrow_mut().suspend()
    }

    fn is_active(&self) -> bool {
        self.inner.borrow().is_active()
    }

    fn set_size(&mut self, width: u32, height: u32) {
        self.inner.borrow_mut().set_size(width, height)
    }

    fn initialize_text_system(&self, doc: &dyn std::any::Any) -> Result<(), String> {
        self.inner.borrow().initialize_text_system(doc)
    }

    fn render<F: FnOnce(&mut Self::ScenePainter<'_>)>(&mut self, draw_fn: F) {
        let ptr = Rc::as_ptr(&self.inner);
        println!("🟢 DxnWindowRenderer::render - Rc ptr: {:p}", ptr);
        self.inner.borrow_mut().render(draw_fn)
    }
}
