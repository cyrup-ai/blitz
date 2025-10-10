use std::sync::{
    Arc,
    atomic::{self, AtomicU64},
};

use anyrender::{WindowHandle, WindowRenderer};
use peniko::Color;
use rustc_hash::FxHashMap;
use vello::{
    AaSupport, RenderParams, Renderer as VelloRenderer, RendererOptions, Scene as VelloScene,
};
use wgpu::{CommandEncoderDescriptor, Features, Limits, PresentMode, TextureViewDescriptor};

use crate::{
    CustomPaintSource, DebugTimer,
    wgpu_context::{DeviceHandle, RenderSurface, WGPUContext},
};
use crate::{DEFAULT_THREADS, GlyphonState, VelloScenePainter};

static PAINT_SOURCE_ID: AtomicU64 = AtomicU64::new(0);

// Simple struct to hold the state of the renderer
struct ActiveRenderState {
    renderer: VelloRenderer,
    surface: RenderSurface<'static>,
}

#[allow(clippy::large_enum_variant)]
enum RenderState {
    Active(ActiveRenderState),
    Suspended,
}

impl RenderState {
    fn current_device_handle(&self) -> Option<&DeviceHandle> {
        let RenderState::Active(state) = self else {
            return None;
        };
        Some(&state.surface.device_handle)
    }
}

pub struct VelloWindowRenderer {
    // The fields MUST be in this order, so that the surface is dropped before the window
    // Window is cached even when suspended so that it can be reused when the app is resumed after being suspended
    render_state: RenderState,
    window_handle: Option<Arc<dyn WindowHandle>>,

    // Vello
    wgpu_context: WGPUContext,
    scene: Option<VelloScene>,
    glyphon_state: Option<GlyphonState>,

    custom_paint_sources: FxHashMap<u64, Box<dyn CustomPaintSource>>,
}
impl VelloWindowRenderer {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self::with_features_and_limits(None, None)
    }

    pub fn with_features_and_limits(features: Option<Features>, limits: Option<Limits>) -> Self {
        let features =
            features.unwrap_or_default() | Features::CLEAR_TEXTURE | Features::PIPELINE_CACHE;
        Self {
            wgpu_context: WGPUContext::with_features_and_limits(Some(features), limits),
            render_state: RenderState::Suspended,
            window_handle: None,
            scene: Some(VelloScene::new()),
            glyphon_state: None,
            custom_paint_sources: FxHashMap::default(),
        }
    }

    pub fn current_device_handle(&self) -> Option<&DeviceHandle> {
        self.render_state.current_device_handle()
    }

    pub fn current_surface_format(&self) -> Option<wgpu::TextureFormat> {
        match &self.render_state {
            RenderState::Active(state) => Some(state.surface.config.format),
            RenderState::Suspended => None,
        }
    }



    pub fn register_custom_paint_source(&mut self, mut source: Box<dyn CustomPaintSource>) -> u64 {
        if let Some(device_handle) = self.render_state.current_device_handle() {
            let instance = &self.wgpu_context.instance;
            source.resume(instance, device_handle);
        }
        let id = PAINT_SOURCE_ID.fetch_add(1, atomic::Ordering::SeqCst);
        self.custom_paint_sources.insert(id, source);
        let self_ptr = self as *const Self;
        println!("🔧🔧 register_custom_paint_source: VelloWindowRenderer instance {:p}, registered source with ID {}, map now has {} sources", 
                 self_ptr, id, self.custom_paint_sources.len());

        id
    }

    pub fn unregister_custom_paint_source(&mut self, id: u64) {
        println!("⚠️ unregister_custom_paint_source called for ID {}", id);
        if let Some(mut source) = self.custom_paint_sources.remove(&id) {
            println!("⚠️ Removed paint source ID {} from map, map now has {} sources", id, self.custom_paint_sources.len());
            source.suspend();
            drop(source);
        } else {
            println!("⚠️ Paint source ID {} not found in map", id);
        }
    }
}

impl WindowRenderer for VelloWindowRenderer {
    type ScenePainter<'a>
        = VelloScenePainter<'a>
    where
        Self: 'a;

    fn is_active(&self) -> bool {
        matches!(self.render_state, RenderState::Active(_))
    }

    fn resume(&mut self, window_handle: Arc<dyn WindowHandle>, width: u32, height: u32) {
        let self_ptr = self as *const Self;
        println!("🟣 VelloWindowRenderer::resume() instance {:p} - custom_paint_sources has {} sources BEFORE resume", 
                 self_ptr, self.custom_paint_sources.len());
        
        let surface = pollster::block_on(self.wgpu_context.create_surface(
            window_handle.clone(),
            width,
            height,
            PresentMode::AutoVsync,
        ))
        .expect("Error creating surface");

        self.window_handle = Some(window_handle);

        let options = RendererOptions {
            antialiasing_support: AaSupport::all(),
            use_cpu: false,
            num_init_threads: DEFAULT_THREADS,
            // TODO: add pipeline cache
            pipeline_cache: None,
        };

        let renderer = VelloRenderer::new(&surface.device_handle.device, options).unwrap();

        self.render_state = RenderState::Active(ActiveRenderState { renderer, surface });

        // Get device handle and initialize custom paint sources
        {
            let device_handle = self.render_state.current_device_handle().unwrap();
            let instance = &self.wgpu_context.instance;
            println!("🟣 VelloWindowRenderer::resume() - resuming {} custom paint sources", 
                     self.custom_paint_sources.len());
            for source in self.custom_paint_sources.values_mut() {
                source.resume(instance, device_handle)
            }
        }
        
        println!("🟣 VelloWindowRenderer::resume() completed - custom_paint_sources has {} sources AFTER resume", 
                 self.custom_paint_sources.len());

        // Initialize glyphon text rendering system and vello resolver
        let RenderState::Active(ref mut state) = self.render_state else {
            panic!("Expected active render state");
        };

        println!("🎯 INITIALIZING GLYPHON STATE in resume()");
        self.glyphon_state = Some(GlyphonState::new(
            &state.surface.device_handle.device,
            &state.surface.device_handle.queue,
            state.surface.config.format,
            width,
            height,
        ));
        println!("✅ GLYPHON STATE INITIALIZED SUCCESSFULLY");

        // Text system initialization will be handled separately
        // since we don't have access to the document here

        // Initialize vello resolver with GPU context for text rendering
        match state.renderer.initialize_resolver(
            &state.surface.device_handle.device,
            &state.surface.device_handle.queue,
            state.surface.config.format,
        ) {
            Ok(()) => {
                log::info!("Successfully initialized vello resolver with GPU context");
            }
            Err(e) => {
                log::error!("Failed to initialize vello resolver: {}", e);
                return;
            }
        }
    }

    fn suspend(&mut self) {
        println!("🔴 VelloWindowRenderer::suspend() called - custom_paint_sources has {} sources", 
                 self.custom_paint_sources.len());
        for source in self.custom_paint_sources.values_mut() {
            source.suspend()
        }
        
        // Clean up vello resolver resources
        if let RenderState::Active(state) = &mut self.render_state {
            state.renderer.cleanup_resolver();
        }
        
        self.glyphon_state = None; // Drop glyphon resources
        self.render_state = RenderState::Suspended;
    }

    fn set_size(&mut self, width: u32, height: u32) {
        if let RenderState::Active(state) = &mut self.render_state {
            state.surface.resize(width, height);
        };
    }

    fn initialize_text_system(&self, doc: &dyn std::any::Any) -> Result<(), String> {
        println!("🔧 VelloWindowRenderer::initialize_text_system called");
        // Try to downcast to BaseDocument
        if let Some(base_doc) = doc.downcast_ref::<blitz_dom::BaseDocument>() {
            println!("🔧 Successfully downcast to BaseDocument");
            if let Some(device_handle) = self.current_device_handle() {
                let format = self.current_surface_format().unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb);
                println!("🎯 INITIALIZING TEXT SYSTEM with GPU context");
                // Initialize text system with GPU context for hardware-accelerated rendering
                // Same parameters as TextRenderer::new() - see lib.rs for detailed explanation
                match pollster::block_on(base_doc.initialize_text_system_with_gpu_context(
                    &device_handle.device,
                    &device_handle.queue,
                    format,
                    wgpu::MultisampleState::default(), // Single-sample, glyphon handles AA
                    None, // No depth stencil for 2D text
                )) {
                    Ok(()) => {
                        println!("✅ TEXT SYSTEM INITIALIZED SUCCESSFULLY");
                        Ok(())
                    }
                    Err(e) => {
                        println!("❌ TEXT SYSTEM INITIALIZATION FAILED: {}", e);
                        Err(format!("Text system initialization failed: {}", e))
                    }
                }
            } else {
                println!("❌ NO GPU DEVICE HANDLE AVAILABLE for text system initialization");
                Err("No GPU device handle available".to_string())
            }
        } else {
            Err("Document is not a BaseDocument".to_string())
        }
    }

    fn render<F: FnOnce(&mut Self::ScenePainter<'_>)>(&mut self, draw_fn: F) {
        log::trace!("VelloWindowRenderer::render() called");
        
        // Get self pointer and log BEFORE any borrows
        let self_ptr = self as *const Self;
        println!("🔧🔧 render: VelloWindowRenderer instance {:p}, creating VelloScenePainter with {} custom paint sources", 
                 self_ptr, self.custom_paint_sources.len());
        for (id, _) in self.custom_paint_sources.iter() {
            println!("🔧🔧   source ID in renderer map: {}", id);
        }
        
        let RenderState::Active(state) = &mut self.render_state else {
            log::warn!("Renderer is not active, skipping render");
            return;
        };

        let surface = &state.surface;
        let device_handle = &surface.device_handle;

        let mut timer = DebugTimer::init();

        let render_params = RenderParams {
            base_color: Color::WHITE,
            width: state.surface.config.width,
            height: state.surface.config.height,
            antialiasing_method: vello::AaConfig::Msaa16,
        };

        // Regenerate the vello scene
        let mut scene = VelloScenePainter {
            inner: self.scene.take().unwrap(),
            renderer: &mut state.renderer,
            custom_paint_sources: &mut self.custom_paint_sources,
            glyphon_state: self.glyphon_state.as_mut(),
        };
        draw_fn(&mut scene);
        self.scene = Some(scene.finish());
        timer.record_time("cmd");

        // Prepare collected text with glyphon BEFORE vello rendering
        if let Some(glyphon) = &mut self.glyphon_state {
            if !glyphon.pending_text_areas.is_empty() {
                // Convert pending text areas to glyphon format
                let text_areas: Vec<glyphon::TextArea> = glyphon
                    .pending_text_areas
                    .iter()
                    .map(|area| glyphon::TextArea {
                        buffer: &area.buffer,
                        left: area.left,
                        top: area.top,
                        scale: area.scale,
                        bounds: area.bounds,
                        default_color: area.color,
                        // Empty slice - custom glyphs are for icons/emoji/special graphics
                        // Standard font-based text rendering doesn't require custom glyphs
                        custom_glyphs: &[],
                    })
                    .collect();

                // Update viewport to match current window size
                glyphon.viewport.update(
                    &device_handle.queue,
                    glyphon::Resolution {
                        width: state.surface.config.width,
                        height: state.surface.config.height,
                    },
                );

                // Prepare glyphs for GPU - this uploads glyph textures to the atlas
                match glyphon.text_renderer.prepare(
                    &device_handle.device,
                    &device_handle.queue,
                    &mut glyphon.font_system.borrow_mut(),
                    &mut glyphon.text_atlas,
                    &glyphon.viewport,
                    text_areas,
                    &mut glyphon.swash_cache,
                ) {
                    Ok(_) => {
                        log::trace!(
                            "Prepared {} text areas for rendering",
                            glyphon.pending_text_areas.len()
                        );
                    }
                    Err(e) => {
                        log::error!("Failed to prepare text for rendering: {:?}", e);
                    }
                }

                // Clear pending areas for next frame
                glyphon.pending_text_areas.clear();
            }
        }
        timer.record_time("text_prepare");

        pollster::block_on(state
            .renderer
            .render_to_texture(
                &device_handle.device,
                &device_handle.queue,
                self.scene.as_ref().unwrap(),
                &surface.target_view,
                &render_params,
            ))
            .expect("failed to render to texture");
        timer.record_time("render");

        // TODO: verify that handling of SurfaceError::Outdated is no longer required
        //
        // let surface_texture = match state.surface.surface.get_current_texture() {
        //     Ok(surface) => surface,
        //     // When resizing too aggresively, the surface can get outdated (another resize) before being rendered into
        //     Err(SurfaceError::Outdated) => return,
        //     Err(_) => panic!("failed to get surface texture"),
        // };

        let surface_texture = state
            .surface
            .surface
            .get_current_texture()
            .expect("failed to get surface texture");

        // Perform the copy
        // (TODO: Does it improve throughput to acquire the surface after the previous texture render has happened?)
        let mut encoder = device_handle
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Surface Blit"),
            });

        state.surface.blitter.copy(
            &device_handle.device,
            &mut encoder,
            &surface.target_view,
            &surface_texture
                .texture
                .create_view(&TextureViewDescriptor::default()),
        );

        // Render glyphon text on top of vello shapes
        if let Some(glyphon) = &mut self.glyphon_state {
            println!("🎯 GLYPHON RENDER: {} pending text areas", glyphon.pending_text_areas.len());
            // Create render pass for text
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Glyphon Text Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_texture
                        .texture
                        .create_view(&TextureViewDescriptor::default()),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // CRITICAL: Load existing content, don't clear!
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Render text with glyphon
            match glyphon.text_renderer.render(
                &glyphon.text_atlas,
                &glyphon.viewport,
                &mut render_pass,
            ) {
                Ok(_) => {
                    log::trace!("Rendered text successfully");
                }
                Err(e) => {
                    log::error!("Failed to render text: {:?}", e);
                }
            }

            // render_pass is dropped here, ending the pass
        }

        device_handle.queue.submit(Some(encoder.finish()));
        surface_texture.present();
        timer.record_time("present");

        if let Err(e) = device_handle.device.poll(wgpu::PollType::wait()) {
            log::warn!("Device poll error: {e}");
        }
        timer.record_time("wait");

        timer.record_time("wait");
        timer.print_times("Frame time: ");

        // static COUNTER: AtomicU64 = AtomicU64::new(0);
        // println!("FRAME {}", COUNTER.fetch_add(1, atomic::Ordering::Relaxed));

        // Empty the Vello scene (memory optimisation)
        self.scene.as_mut().unwrap().reset();
    }
}
