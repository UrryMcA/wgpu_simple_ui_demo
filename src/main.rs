mod device;
mod bmfont;
mod texture_loader_adapter;

use anyhow::Result;
use wgpu_simple_ui::{DefaultPrimitives, UiRenderer, common::types::{Alignment, EdgeInsets, Size, UColor}, outline_rect::OutlineRect, panel::Panel, *};
use wgpu_simple_ui_winit::window_event_to_ui_event;
use std::sync::Arc;
use winit::{
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
    application::ApplicationHandler,
};
use wgpu::{Surface, SurfaceConfiguration, CurrentSurfaceTexture};
use device::GraphicsContext;

use crate::{bmfont::bmfont_adapter::BmFontLoaderAdapter, texture_loader_adapter::FileTextureLoader};

struct App {
    window: Option<Arc<Window>>,          // Arc для статического времени жизни
    surface: Option<Surface<'static>>,    // 'static разрешено благодаря Arc
    gfx: Option<GraphicsContext>,
    config: Option<SurfaceConfiguration>,
    ui_renderer: Option<UiRenderer>,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            surface: None,
            gfx: None,
            config: None,
            ui_renderer: None,
        }
    }

    fn init(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title("Minimal wgpu 29")
                    .with_inner_size(winit::dpi::PhysicalSize::new(1024, 768)),
            )?,
        );
        self.window = Some(window.clone());

        let gfx = pollster::block_on(GraphicsContext::new())?;
        self.gfx = Some(gfx);
        let gfx = self.gfx.as_ref().unwrap();

        let surface = gfx.instance.create_surface(window)?;
        self.surface = Some(surface);
        let surface_ref = self.surface.as_ref().unwrap();

        let caps = surface_ref.get_capabilities(&gfx.adapter);
        let size = self.window.as_ref().unwrap().inner_size();
        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: caps.formats[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface_ref.configure(&gfx.device, &config);



        // --- Создание UI рендерера ---
        let primitives = Box::new(DefaultPrimitives);
        let mut ui_renderer = UiRenderer::new(
            &gfx.device,
            &gfx.queue,
            config.format,
            size.width,
            size.height,
            primitives,
        );

        // Загружаем шрифты через адаптер
        let font_loader = BmFontLoaderAdapter::new();
        if !ui_renderer.load_font("default", &font_loader) {
            eprintln!("Warning: failed to load default font, text may not render");
        }
        if !ui_renderer.load_font("title", &font_loader) {
            eprintln!("Warning: failed to load title font");
        }

        // Загружаем текстуру (опционально)
        let texture_loader = FileTextureLoader;
        let _texture_id = ui_renderer.load_texture("assets/icon.png", &texture_loader);

        // Строим дерево виджетов
        let mut ui_widget = build_test_ui();
        let mut ui_manager = ui_renderer.ui_manager();
        ui_manager.set_root(ui_widget.create_render_object());
        ui_manager.layout(Size::new(size.width as f32, size.height as f32));

        self.ui_renderer = Some(ui_renderer);
        self.config = Some(config);

        Ok(())
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            if let (Some(surface), Some(config), Some(gfx)) = (
                self.surface.as_mut(),
                self.config.as_mut(),
                self.gfx.as_ref(),
            ) {
                config.width = new_size.width;
                config.height = new_size.height;
                surface.configure(&gfx.device, config);
/*
                renderer.resize(new_size.width, new_size.height);
                renderer.ui_manager().layout(Size::new(new_size.width as f32, new_size.height as f32));
                 */
            if let Some(renderer) = self.ui_renderer.as_mut() {
                renderer.resize(new_size.width, new_size.height);
            }
            }
        }
    }

    fn render(&mut self) -> Result<()> {
        let gfx = self.gfx.as_ref().unwrap();
        let surface = self.surface.as_ref().unwrap();
        let config = self.config.as_ref().unwrap();

        // Правильная обработка CurrentSurfaceTexture (wgpu 29)
        let frame = match surface.get_current_texture() {
            CurrentSurfaceTexture::Success(texture) => texture,
            CurrentSurfaceTexture::Suboptimal(texture) => {
                surface.configure(&gfx.device, config);
                texture
            }
            CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            CurrentSurfaceTexture::Outdated => {
                surface.configure(&gfx.device, config);
                return Ok(());
            }
            CurrentSurfaceTexture::Lost => {
                return Err(anyhow::anyhow!("Surface lost"));
            }
            CurrentSurfaceTexture::Validation => {
                return Err(anyhow::anyhow!("Validation error in get_current_texture"));
            }
        };
        let view = frame.texture.create_view(&Default::default());

        let mut encoder = gfx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

             //renderer.draw_commands_into(render_pass);
        }

        let renderer: &mut UiRenderer = self.ui_renderer.as_mut().unwrap();
        renderer.render(&mut encoder, &view);

        gfx.queue.submit(Some(encoder.finish()));
        frame.present();

        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(e) = self.init(event_loop) {
            eprintln!("Initialization error: {}", e);
            event_loop.exit();
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {

        let renderer: &mut UiRenderer = self.ui_renderer.as_mut().unwrap();
        let mut tr_event = window_event_to_ui_event(&event);
        match tr_event {
            Some(value) =>{
                renderer.ui_manager().process_event(&value);
            },
            None => {},
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.render() {
                    eprintln!("Render error: {}", e);
                    event_loop.exit();
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::Resized(size) => self.resize(size),
            _ => {}
        }
    }
}

fn main() -> Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}


fn build_test_ui2() -> wgpu_simple_ui::ui::Container {
    let button = Button::new("Click me!")
        .padding(EdgeInsets::all(16.0))
        .margin(EdgeInsets::all(1.0))
        .color(UColor::new(0.2, 0.5, 0.8, 1.0))
        .corner_radius(16.0)
        .on_click(|| {
            println!("Button clicked!");
        });

    let label = Label::new("Hello from wgpu UI")
//    let label = Label::new("E")
        .font_size(42.0)
        .color(UColor::new(0.2, 1.0, 1.0, 1.0))
        .margin(EdgeInsets::all(8.0));

    Container::vertical()
        .alignment(wgpu_simple_ui::common::types::Alignment::Center)
        .spacing(20.0)
        .add_child(Box::new(label))
        .add_child(Box::new(button))
}

fn build_test_ui() -> wgpu_simple_ui::ui::Container {
    let button = Button::new("Click me!")
        .padding(EdgeInsets::all(16.0))
        .margin(EdgeInsets::all(10.0))
        .color(UColor::new(0.2, 0.5, 0.8, 1.0))
        .corner_radius(16.0)
        .on_click(|| {
            println!("Button clicked!");
        });

    let label = Label::new("Hello from wgpu UI")
        .font_size(42.0)
        .color(UColor::new(0.2, 1.0, 1.0, 1.0))
        .margin(EdgeInsets::all(8.0));

    // Новый виджет: контур скруглённого прямоугольника
    let outline_rect = OutlineRect::new(200.0, 100.0)
        .corner_radius(20.0)
        .thickness(4.0)
        .color(UColor::new(1.0, 0.8, 0.0, 1.0)) // оранжевый
        .margin(EdgeInsets::all(20.0));

    // Ещё один outline для демонстрации
    let outline_rect2 = OutlineRect::new(150.0, 80.0)
        .corner_radius(12.0)
        .thickness(3.0)
        .color(UColor::new(0.0, 1.0, 0.5, 1.0)) // зелёный
        .margin(EdgeInsets::all(15.0));

    use wgpu_simple_ui::ui::canvas::{Canvas, CanvasItem};
    use wgpu_simple_ui::common::types::{Rect, Line, UColor};

    let mut canvas = Canvas::new(800.0, 300.0)
        .margin(EdgeInsets::all(20.0))
        .on_click(|p| println!("Clicked at: {:?}", p));

    // Рамка графика
    canvas.push_item(CanvasItem::OutlineRect {
        rect: Rect::new(0.0, 0.0, 800.0, 300.0),
        radius: 8.0,
        thickness: 2.0,
        color: UColor::new(0.5, 0.5, 0.5, 1.0),
    });

    // 500 столбиков гистограммы
    for i in 0..500 {
        let x = i as f32 * 1.5 + 10.0;
        let h = ((i as f32 * 0.05).sin() * 0.5 + 0.5) * 250.0;
        canvas.push_item(CanvasItem::Rect {
            rect: Rect::new(x, 300.0 - h, 1.2, h),
            color: UColor::new(0.2, 0.6, 1.0, 1.0),
        });
    }
        // Диагональная линия
    canvas.push_item(CanvasItem::Line {
        line: Line::new(0.0, 0.0, 800.0, 300.0, 1.5),
        color: UColor::new(1.0, 0.3, 0.3, 1.0),
    });


    wgpu_simple_ui::ui::Container::vertical()
        .alignment(wgpu_simple_ui::common::types::Alignment::Center)
        .spacing(20.0)
        .add_child(Box::new(label))
        .add_child(Box::new(button))
        .add_child(Box::new(outline_rect))
        .add_child(Box::new(outline_rect2))
         .add_child(Box::new(canvas))
}        

