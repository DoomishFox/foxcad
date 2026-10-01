mod command;
mod ui;
mod viewport;

use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

#[derive(Debug)]
enum Wake {
    Repaint(Instant),
    DeviceLost(String),
}
struct Gpu {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    gui: egui_wgpu::Renderer,
    strokes: fox_graphics::StrokeRenderer,
    adapter: String,
}
impl Gpu {
    async fn new(
        window: Arc<Window>,
        proxy: EventLoopProxy<Wake>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(window.clone()),
        ));
        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("FoxCAD + egui shared device"),
                ..Default::default()
            })
            .await?;
        device.set_device_lost_callback(move |reason, message| {
            if reason != wgpu::DeviceLostReason::Destroyed {
                let _ = proxy.send_event(Wake::DeviceLost(message));
            }
        });
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("No supported surface configuration")?;
        config.format = surface
            .get_capabilities(&adapter)
            .formats
            .into_iter()
            .find(|f| !f.is_srgb())
            .unwrap_or(config.format);
        config.present_mode = wgpu::PresentMode::Fifo;
        surface.configure(&device, &config);
        let gui = egui_wgpu::Renderer::new(&device, config.format, Default::default());
        let strokes = fox_graphics::StrokeRenderer::new(&device, config.format);
        let info = adapter.get_info();
        let adapter = format!("{} / {:?}", info.name, info.backend);
        eprintln!("Shared GPU: {adapter}; format {:?}", config.format);
        Ok(Self {
            instance,
            surface,
            device,
            queue,
            config,
            gui,
            strokes,
            adapter,
        })
    }
    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width > 0 && size.height > 0 {
            self.config.width = size.width;
            self.config.height = size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }
}
struct Desktop {
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    context: egui::Context,
    input: Option<egui_winit::State>,
    ui: ui::DraftUi,
    proxy: EventLoopProxy<Wake>,
    repaint: Option<Instant>,
    smoke_frames: Option<u32>,
    frames: u32,
    failure: Option<String>,
    smoke_deadline: Option<Instant>,
}
impl Desktop {
    fn fail(&mut self, event_loop: &ActiveEventLoop, message: String) {
        eprintln!("FoxCAD: {message}");
        self.failure = Some(message);
        event_loop.exit();
    }
    fn draw(&mut self, event_loop: &ActiveEventLoop) {
        let Some(window) = self.window.clone() else {
            return;
        };
        if window.inner_size().width == 0 || window.inner_size().height == 0 {
            return;
        }
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };
        let mut reconfigure = false;
        let frame = match gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                reconfigure = true;
                window.request_redraw();
                frame
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                gpu.resize(window.inner_size());
                window.request_redraw();
                return;
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                match gpu.instance.create_surface(window.clone()) {
                    Ok(surface) => {
                        gpu.surface = surface;
                        gpu.resize(window.inner_size());
                        window.request_redraw();
                    }
                    Err(error) => {
                        self.fail(event_loop, format!("Surface recovery failed: {error}"))
                    }
                }
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout => {
                self.repaint = Some(Instant::now() + Duration::from_millis(100));
                return;
            }
            wgpu::CurrentSurfaceTexture::Occluded => return,
            wgpu::CurrentSurfaceTexture::Validation => {
                self.fail(event_loop, "GPU surface validation failed".into());
                return;
            }
        };
        let input = self.input.as_mut().unwrap();
        let mut output = self.context.run_ui(input.take_egui_input(&window), |root| {
            self.ui.show(root, &gpu.adapter)
        });
        input.handle_platform_output_with_event_loop(&window, event_loop, output.platform_output);
        let jobs = self
            .context
            .tessellate(output.shapes, output.pixels_per_point);
        for (id, delta) in &output.textures_delta.set {
            for image in delta {
                gpu.gui.update_texture(&gpu.device, &gpu.queue, *id, image);
            }
        }
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [gpu.config.width, gpu.config.height],
            pixels_per_point: output.pixels_per_point,
        };
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("FoxCAD frame"),
            });
        let buffers = gpu
            .gui
            .update_buffers(&gpu.device, &gpu.queue, &mut encoder, &jobs, &screen);
        gpu.strokes.prepare(
            &gpu.device,
            &gpu.queue,
            &self.ui.strokes,
            [self.ui.rect.width(), self.ui.rect.height()],
        );
        let view = frame.texture.create_view(&Default::default());
        {
            let attachments = [Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.047,
                        g: 0.063,
                        b: 0.086,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })];
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("CAD viewport"),
                color_attachments: &attachments,
                ..Default::default()
            });
            let ppp = output.pixels_per_point;
            let rect = self.ui.rect;
            let x = (rect.min.x * ppp)
                .round()
                .clamp(0.0, gpu.config.width as f32) as u32;
            let y = (rect.min.y * ppp)
                .round()
                .clamp(0.0, gpu.config.height as f32) as u32;
            let right = (rect.max.x * ppp)
                .round()
                .clamp(x as f32, gpu.config.width as f32) as u32;
            let bottom = (rect.max.y * ppp)
                .round()
                .clamp(y as f32, gpu.config.height as f32) as u32;
            if right > x && bottom > y {
                pass.set_viewport(
                    x as f32,
                    y as f32,
                    (right - x) as f32,
                    (bottom - y) as f32,
                    0.0,
                    1.0,
                );
                pass.set_scissor_rect(x, y, right - x, bottom - y);
                gpu.strokes.render(&mut pass);
            }
        }
        {
            let attachments = [Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })];
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui overlay"),
                color_attachments: &attachments,
                ..Default::default()
            });
            gpu.gui.render(&mut pass.forget_lifetime(), &jobs, &screen);
        }
        gpu.queue
            .submit(buffers.into_iter().chain(std::iter::once(encoder.finish())));
        window.pre_present_notify();
        gpu.queue.present(frame);
        if reconfigure {
            gpu.resize(window.inner_size());
        }
        for id in &output.textures_delta.free {
            gpu.gui.free_texture(id);
        }
        output.textures_delta.clear();
        self.frames += 1;
        if let Some(limit) = self.smoke_frames {
            if self.frames >= limit {
                eprintln!(
                    "SMOKE PASS: {} frames presented; {:.2} pixels/point",
                    self.frames, output.pixels_per_point
                );
                event_loop.exit();
            } else {
                window.request_redraw();
            }
        }
    }
}
impl ApplicationHandler<Wake> for Desktop {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let result = (|| -> Result<_, Box<dyn std::error::Error>> {
            let window = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("FoxCAD — Phase 0")
                        .with_inner_size(winit::dpi::LogicalSize::new(1200.0, 800.0))
                        .with_min_inner_size(winit::dpi::LogicalSize::new(800.0, 540.0)),
                )?,
            );
            let gpu = pollster::block_on(Gpu::new(window.clone(), self.proxy.clone()))?;
            let input = egui_winit::State::new(
                self.context.clone(),
                egui::ViewportId::ROOT,
                window.as_ref(),
                Some(window.scale_factor() as f32),
                window.theme(),
                Some(gpu.device.limits().max_texture_dimension_2d as usize),
            );
            Ok((window, gpu, input))
        })();
        match result {
            Ok((window, gpu, input)) => {
                window.request_redraw();
                self.window = Some(window);
                self.gpu = Some(gpu);
                self.input = Some(input);
            }
            Err(error) => self.fail(event_loop, format!("Initialization failed: {error}")),
        }
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: Wake) {
        match event {
            Wake::Repaint(when) => {
                self.repaint = Some(self.repaint.map_or(when, |old| old.min(when)))
            }
            Wake::DeviceLost(message) => {
                eprintln!("Recreating GPU after device loss: {message}");
                if let Some(window) = self.window.clone() {
                    match pollster::block_on(Gpu::new(window.clone(), self.proxy.clone())) {
                        Ok(gpu) => {
                            self.gpu = Some(gpu);
                            // A fresh egui context regenerates its font atlas for the new device.
                            self.context = make_context(self.proxy.clone());
                            self.input = Some(egui_winit::State::new(
                                self.context.clone(),
                                egui::ViewportId::ROOT,
                                window.as_ref(),
                                Some(window.scale_factor() as f32),
                                window.theme(),
                                None,
                            ));
                            window.request_redraw();
                        }
                        Err(error) => {
                            self.fail(event_loop, format!("GPU recovery failed: {error}"))
                        }
                    }
                }
            }
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.clone().filter(|w| w.id() == id) else {
            return;
        };
        let response = self
            .input
            .as_mut()
            .unwrap()
            .on_window_event(&window, &event);
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size);
                }
                window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(window.inner_size());
                }
                window.request_redraw();
            }
            WindowEvent::Occluded(false) => window.request_redraw(),
            WindowEvent::RedrawRequested => self.draw(event_loop),
            _ => {
                if response.repaint {
                    window.request_redraw();
                }
            }
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.smoke_deadline.is_some_and(|t| Instant::now() >= t) {
            self.fail(
                event_loop,
                "Smoke test timed out before presenting frames".into(),
            );
            return;
        }
        if self.repaint.is_some_and(|t| Instant::now() >= t) {
            self.repaint = None;
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        let deadline = self.repaint.into_iter().chain(self.smoke_deadline).min();
        event_loop.set_control_flow(deadline.map_or(ControlFlow::Wait, ControlFlow::WaitUntil));
    }
}
fn make_context(proxy: EventLoopProxy<Wake>) -> egui::Context {
    let context = egui::Context::default();
    context.set_theme(egui::Theme::Dark);
    context.set_visuals(egui::Visuals::dark());
    context.options_mut(|o| o.max_passes = std::num::NonZeroUsize::new(1).unwrap());
    context.set_request_repaint_callback(move |request| {
        if let Some(when) = Instant::now().checked_add(request.delay) {
            let _ = proxy.send_event(Wake::Repaint(when));
        }
    });
    context
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args().any(|s| s == "--smoke-test");
    let event_loop = EventLoop::<Wake>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let context = make_context(proxy.clone());
    let mut app = Desktop {
        window: None,
        gpu: None,
        context,
        input: None,
        ui: Default::default(),
        proxy,
        repaint: None,
        smoke_frames: smoke.then_some(4),
        frames: 0,
        failure: None,
        smoke_deadline: smoke.then(|| Instant::now() + Duration::from_secs(30)),
    };
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.failure {
        return Err(error.into());
    }
    Ok(())
}
