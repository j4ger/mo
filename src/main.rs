use std::{num::NonZeroUsize, sync::Arc};

use anyhow::{Ok, Result};
use vello::{
    util::{RenderContext, RenderSurface},
    wgpu::PresentMode,
    AaConfig, Renderer, RendererOptions,
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

struct RenderState<'s> {
    surface: RenderSurface<'s>,
    window: Arc<Window>,
}

struct MoApp<'s> {
    state: Option<RenderState<'s>>,
    context: RenderContext,
    cached_window: Option<Arc<Window>>,
    renderers: Vec<Option<Renderer>>,
}

impl<'s> ApplicationHandler for MoApp<'s> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        let window = self.cached_window.take().unwrap_or_else(|| {
            Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_inner_size(LogicalSize::new(1000, 500))
                            .with_resizable(true)
                            .with_title("Mo demo"),
                    )
                    .unwrap(),
            )
        });
        let size = window.inner_size();
        let present_mode = PresentMode::AutoNoVsync;
        let surface_texture =
            self.context
                .create_surface(window.clone(), size.width, size.height, present_mode);
        let surface = pollster::block_on(surface_texture).unwrap();
        self.state = {
            let render_state = RenderState { window, surface };
            self.renderers
                .resize_with(self.context.devices.len(), || None);
            let id = render_state.surface.dev_id;
            self.renderers[id].get_or_insert_with(|| {
                Renderer::new(
                    &self.context.devices[id].device,
                    RendererOptions {
                        surface_format: Some(render_state.surface.format),
                        use_cpu: false,
                        antialiasing_support: [AaConfig::Area, AaConfig::Msaa8, AaConfig::Msaa16]
                            .iter()
                            .copied()
                            .collect(),
                        num_init_threads: NonZeroUsize::new(8),
                    },
                )
                .unwrap()
            });
            Some(render_state)
        };
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(RenderState { surface, window }) = &mut self.state {
                    self.context
                        .resize_surface(surface, size.width, size.height);
                    window.request_redraw();
                };
            }
            WindowEvent::RedrawRequested => {}
            _ => (),
        }
    }

    fn suspended(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(render_state) = self.state.take() {
            self.cached_window = Some(render_state.window);
        }
    }
}

fn main() -> Result<()> {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut renderers = Vec::new();
    let context = RenderContext::new();

    let mut app = MoApp {
        renderers,
        state: None,
        context,
        cached_window: None,
    };

    event_loop.run_app(&mut app);
    Ok(())
}
