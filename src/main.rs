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
        //let _texture_id = ui_renderer.load_texture("assets/icon.png", &texture_loader);
        // Загружаем текстуры
        let bg_id = ui_renderer.load_texture("assets/ui_bg.png", &texture_loader).unwrap_or(0);
        let icon_id = ui_renderer.load_texture("assets/icon.png", &texture_loader).unwrap_or(0);

        // ===================================================================================
        // Строим дерево виджетов
        //let mut ui_widget = build_test_ui();
        // Строим дерево виджетов, передавая ID текстур
        let mut ui_widget = build_test_ui_bg(bg_id, icon_id);
        
 
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

            /*
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
 */
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

fn build_test_ui_3() -> wgpu_simple_ui::ui::Container {
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

fn build_test_ui() -> wgpu_simple_ui::ui::Container {
    use wgpu_simple_ui::ui::{Button, Label, Container, canvas::{Canvas, CanvasItem}};
    use wgpu_simple_ui::common::types::{Rect, Line, UColor, EdgeInsets, Alignment};
    use wgpu_simple_ui::outline_rect::OutlineRect;
    use wgpu_simple_ui::panel::Panel;

    // === ЗАГОЛОВОК ===
    let header = Label::new("🧪 UI Stress Test Suite")
        .font_size(28.0)
        .color(UColor::new(1.0, 0.9, 0.2, 1.0))
        .margin(EdgeInsets::new(10.0, 10.0, 20.0, 10.0));

    // === ГРУППА КНОПОК: Цветовые темы ===
    let mut color_buttons = Container::horizontal()
        .spacing(8.0)
        .alignment(Alignment::Center);

    let colors = [
        ("🔴 Red", UColor::new(0.9, 0.2, 0.2, 1.0)),
        ("🟢 Green", UColor::new(0.2, 0.8, 0.3, 1.0)),
        ("🔵 Blue", UColor::new(0.2, 0.5, 0.9, 1.0)),
        ("🟡 Yellow", UColor::new(0.95, 0.85, 0.1, 1.0)),
        ("🟣 Purple", UColor::new(0.7, 0.3, 0.9, 1.0)),
        ("⚪ White", UColor::new(0.95, 0.95, 0.95, 1.0)),
        ("⚫ Black", UColor::new(0.1, 0.1, 0.15, 1.0)),
        ("🟠 Orange", UColor::new(1.0, 0.5, 0.1, 1.0)),
    ];

    for (text, color) in colors {
        let btn = Button::new(text)
            .padding(EdgeInsets::all(12.0))
            .color(color)
            .corner_radius(8.0)
            .on_click(move || println!("Clicked: {}", text));
        // ✅ ВАЖНО: перезаписываем контейнер, т.к. add_child consumes self
        color_buttons = color_buttons.add_child(Box::new(btn));
    }

    // === ГРУППА КНОПОК: Размеры ===
    let size_label = Label::new("📏 Sizes:")
        .font_size(16.0)
        .color(UColor::new(0.8, 0.8, 0.9, 1.0))
        .margin(EdgeInsets::all(8.0));

    let mut size_buttons = Container::horizontal().spacing(6.0).alignment(Alignment::Center);
    for (label, padding, radius) in [
        ("XS", 6.0, 4.0), ("S", 10.0, 6.0), ("M", 14.0, 8.0), 
        ("L", 20.0, 12.0), ("XL", 28.0, 16.0),
    ] {
        let btn = Button::new(label)
            .padding(EdgeInsets::all(padding))
            .corner_radius(radius)
            .color(UColor::new(0.3, 0.6, 0.8, 1.0))
            .on_click(move || println!("Size {} clicked", label));
        size_buttons = size_buttons.add_child(Box::new(btn));
    }

    // === ГРУППА КНОПОК: Стили границ ===
    let style_label = Label::new("🎨 Border Styles:")
        .font_size(16.0)
        .color(UColor::new(0.8, 0.8, 0.9, 1.0))
        .margin(EdgeInsets::all(8.0));

    let mut style_buttons = Container::horizontal().spacing(8.0);
    for (name, radius, outline) in [
        ("Sharp", 0.0, false),
        ("Round", 20.0, false),
        ("Pill", 50.0, false),
        ("Outlined", 8.0, true),
        ("Thick", 12.0, true),
    ] {
        let mut btn = Button::new(name)
            .padding(EdgeInsets::all(10.0))
            .corner_radius(radius)
            .color(if outline { UColor::new(0.15, 0.15, 0.25, 1.0) } else { UColor::new(0.4, 0.5, 0.7, 1.0) })
            .on_click(move || println!("Style {} clicked", name));
        
        if outline {
            btn = btn.border_color(UColor::new(0.9, 0.7, 0.3, 1.0))
                     .border_thickness(if name == "Thick" { 3.0 } else { 1.5 });
        }
        style_buttons = style_buttons.add_child(Box::new(btn));
    }

    // === GRID КНОПОК: 5×4 матрица для теста батчинга ===
    let grid_label = Label::new("🔢 Button Grid (5×4):")
        .font_size(16.0)
        .color(UColor::new(0.8, 0.8, 0.9, 1.0))
        .margin(EdgeInsets::all(10.0));

    let mut grid_container = Container::vertical().spacing(6.0);
    for row in 0..2 {
        let mut row_container = Container::horizontal().spacing(6.0);
        for col in 0..8 {
            let idx = row * 5 + col + 1;
            let hue = (idx as f32 * 25.0) / 360.0;
            // Fallback для from_hsv, если его ещё нет в types.rs
            let btn_color = UColor::new(0.2 + hue * 0.6, 0.3 + hue * 0.4, 0.9 - hue * 0.3, 1.0);
            let btn = Button::new(&format!("{:02}", idx))
                .padding(EdgeInsets::all(10.0))
                .corner_radius(6.0)
                .color(btn_color)
                .on_click(move || println!("Grid button #{} clicked", idx));
            row_container = row_container.add_child(Box::new(btn));
        }
        grid_container = grid_container.add_child(Box::new(row_container));
    }

    // === OUTLINE RECTS ===
    let outlines_label = Label::new("🔲 Outline Primitives:")
        .font_size(16.0)
        .color(UColor::new(0.8, 0.8, 0.9, 1.0))
        .margin(EdgeInsets::all(10.0));

    let mut outlines_row = Container::horizontal().spacing(15.0).alignment(Alignment::Center);
    
    let outline1 = OutlineRect::new(120.0, 60.0)
        .corner_radius(0.0).thickness(2.0).color(UColor::new(1.0, 0.3, 0.3, 1.0))
        .margin(EdgeInsets::new(10.0, 15.0, 10.0, 15.0));
    let outline2 = OutlineRect::new(120.0, 60.0)
        .corner_radius(12.0).thickness(3.0).color(UColor::new(0.3, 1.0, 0.5, 1.0))
        .margin(EdgeInsets::all(10.0));
    let outline3 = OutlineRect::new(120.0, 60.0)
        .corner_radius(30.0).thickness(4.0).color(UColor::new(0.4, 0.6, 1.0, 1.0))
        .margin(EdgeInsets::all(10.0));
    let outline4 = OutlineRect::new(120.0, 60.0)
        .corner_radius(60.0).thickness(2.5).color(UColor::new(0.9, 0.4, 1.0, 1.0))
        .margin(EdgeInsets::all(10.0));

    outlines_row = outlines_row.add_child(Box::new(outline1));
    outlines_row = outlines_row.add_child(Box::new(outline2));
    outlines_row = outlines_row.add_child(Box::new(outline3));
    outlines_row = outlines_row.add_child(Box::new(outline4));

    // === CANVAS ===
    let canvas_label = Label::new("📊 Canvas Graphics:")
        .font_size(16.0)
        .color(UColor::new(0.8, 0.8, 0.9, 1.0))
        .margin(EdgeInsets::all(10.0));

    let mut canvas = Canvas::new(700.0, 100.0)
        .margin(EdgeInsets::new(5.0, 10.0, 5.0, 10.0))
        .on_click(|p| println!("Canvas clicked at {:?}", p));

    for i in 0..35 {
        let x = i as f32 * 20.0;
        canvas.push_item(CanvasItem::Line { line: Line::new(x, 0.0, x, 100.0, 0.5), color: UColor::new(0.2, 0.2, 0.3, 0.3) });
    }
    for i in 0..10 {
        let y = i as f32 * 20.0;
        canvas.push_item(CanvasItem::Line { line: Line::new(0.0, y, 700.0, y, 0.5), color: UColor::new(0.2, 0.2, 0.3, 0.3) });
    }

    for i in 0..100 {
        let x = 10.0 + i as f32 * 6.8;
        let h = ((i as f32 * 0.15).sin() * 0.5 + 0.5) * 100.0 + 20.0;
        let hue = i as f32 / 100.0;
        canvas.push_item(CanvasItem::Rect {
            rect: Rect::new(x, 200.0 - h, 5.5, h),
            color: UColor::new(0.2 + hue * 0.6, 0.3, 0.9, 1.0),
        });
    }

    let mut prev_point: Option<(f32, f32)> = None;
    for i in 0..200 {
        let x = 10.0 + i as f32 * 3.45;
        let y = 100.0 + (i as f32 * 0.08).sin() * 70.0;
        if let Some((px, py)) = prev_point {
            canvas.push_item(CanvasItem::Line { line: Line::new(px, py, x, y, 2.0), color: UColor::new(1.0, 0.4, 0.6, 0.9) });
        }
        prev_point = Some((x, y));
    }

    canvas.push_item(CanvasItem::OutlineRect {
        rect: Rect::new(0.0, 0.0, 700.0, 200.0),
        radius: 10.0, thickness: 2.0,
        color: UColor::new(0.7, 0.7, 0.85, 1.0),
    });

    // === PANEL ===
    let panel_label = Label::new("📦 Nested Panel:")
        .font_size(16.0)
        .color(UColor::new(0.8, 0.8, 0.9, 1.0))
        .margin(EdgeInsets::all(10.0));

    let mut panel_content = Container::vertical().spacing(8.0);
    for i in 1..=4 {
        let btn = Button::new(&format!("Panel Button #{}", i))
            .padding(EdgeInsets::all(8.0))
            .corner_radius(6.0)
            .color(UColor::new(0.25, 0.45, 0.75, 1.0))
            .on_click(move || println!("Panel button {} clicked", i));
        panel_content = panel_content.add_child(Box::new(btn));
    }

    // Примечание: используется обновлённый Panel из предыдущего ответа (.content())
/*    let panel = Panel::new()
        .padding(EdgeInsets::new(15.0, 20.0, 15.0, 20.0))
        .corner_radius(12.0)
        .color(UColor::new(0.12, 0.15, 0.22, 0.95))
        .border_color(UColor::new(0.4, 0.5, 0.7, 1.0))
        .border_thickness(1.5)
        .content(Box::new(panel_content));
    */
    let panel_content = Container::vertical()
        .spacing(8.0)
        .add_child(Box::new(Label::new("Panel Content").font_size(14.0).color(UColor::new(0.9, 0.9, 0.9, 1.0))));

    let panel = Panel::new(Box::new(panel_content))
        .padding(EdgeInsets::new(15.0, 20.0, 15.0, 20.0))
        .corner_radius(12.0)
        .background(UColor::new(0.12, 0.15, 0.22, 0.95))
        .margin(EdgeInsets::all(10.0));

    // === FOOTER ===
    let footer = Label::new("✅ Render test complete — check batching, scissor, and GPU buffer handling")
        .font_size(16.0)
        .color(UColor::new(0.6, 0.7, 0.85, 1.0))
        .margin(EdgeInsets::new(0.0, 20.0, 10.0, 10.0));

    // === СБОРКА ИЕРАРХИИ ===
    Container::vertical()
        .alignment(Alignment::Center)
        .spacing(12.0)
        .padding(EdgeInsets::new(15.0, 15.0, 15.0, 15.0))
        .add_child(Box::new(header))
        .add_child(Box::new(color_buttons))
        .add_child(Box::new(size_label))
        .add_child(Box::new(size_buttons))
        .add_child(Box::new(style_label))
        .add_child(Box::new(style_buttons))
        .add_child(Box::new(grid_label))
        .add_child(Box::new(grid_container))
        .add_child(Box::new(outlines_label))
        .add_child(Box::new(outlines_row))
        .add_child(Box::new(canvas_label))
        .add_child(Box::new(canvas))
        .add_child(Box::new(panel_label))
        .add_child(Box::new(panel))
        .add_child(Box::new(footer))
}


fn build_test_ui_bg(bg_id: u64, icon_id: u64) -> wgpu_simple_ui::ui::Container {
    use wgpu_simple_ui::ui::{Button, Label, Container, icon_button::IconButton};
    use wgpu_simple_ui::panel::Panel;
    use wgpu_simple_ui::common::types::{BackgroundFit, EdgeInsets, Size, UColor, Alignment};

    // 📌 Заголовки секций
    let sec1_label = Label::new("1️⃣ Solid Baseline (Цветной фон)").font_size(16.0).color(UColor::new(0.7, 0.8, 1.0, 1.0)).margin(EdgeInsets::all(5.0));
    let sec2_label = Label::new("2️⃣ BackgroundFit (Растяжение, Черепица, Cover)").font_size(16.0).color(UColor::new(0.7, 0.8, 1.0, 1.0)).margin(EdgeInsets::all(5.0));
    let sec3_label = Label::new("3️⃣ Transparent PNG Overlay (Цвет + PNG)").font_size(16.0).color(UColor::new(0.7, 0.8, 1.0, 1.0)).margin(EdgeInsets::all(5.0));
    let sec4_label = Label::new("4️⃣ IconButtons (С оверлеем и без)").font_size(16.0).color(UColor::new(0.7, 0.8, 1.0, 1.0)).margin(EdgeInsets::all(5.0));

    // ================= 1️⃣ SOLID BASELINE =================
    let solid_btn = Button::new("Solid Button")
        .padding(EdgeInsets::all(12.0))
        .color(UColor::new(0.2, 0.4, 0.7, 1.0))
        .corner_radius(8.0)
        .on_click(|| println!("Solid btn clicked"));

    let solid_panel = Panel::new(Box::new(Label::new("Solid Panel").font_size(14.0).color(UColor::new(0.9, 0.9, 0.9, 1.0))))
        .background(UColor::new(0.15, 0.15, 0.2, 1.0))
        .corner_radius(12.0)
        .padding(EdgeInsets::all(16.0));

    let sec1_row = Container::horizontal()
        .spacing(10.0)
        .alignment(Alignment::Center)
        .add_child(Box::new(solid_btn))
        .add_child(Box::new(solid_panel));

    // ================= 2️⃣ BACKGROUNDFIT TESTS =================
    let panel_stretch = Panel::new(Box::new(Label::new("Stretch").font_size(14.0).color(UColor::new(1.0, 1.0, 1.0, 1.0))))
        .background_texture_overlay(bg_id, BackgroundFit::Stretch, Some(UColor::new(1.0, 1.0, 1.0, 0.8)))
        .corner_radius(10.0).padding(EdgeInsets::all(20.0));

    let panel_tile = Panel::new(Box::new(Label::new("Tile (0.5x)").font_size(14.0).color(UColor::new(1.0, 1.0, 1.0, 1.0))))
        .background_texture_overlay(bg_id, BackgroundFit::Tile { scale: 0.5 }, Some(UColor::new(1.0, 1.0, 1.0, 0.9)))
        .corner_radius(10.0).padding(EdgeInsets::all(20.0));

    let panel_cover = Panel::new(Box::new(Label::new("Cover").font_size(14.0).color(UColor::new(1.0, 1.0, 1.0, 1.0))))
        .background_texture_overlay(bg_id, BackgroundFit::Cover, None)
        .corner_radius(10.0).padding(EdgeInsets::all(20.0));

    let sec2_row = Container::horizontal()
        .spacing(10.0)
        .alignment(Alignment::Center)
        .add_child(Box::new(panel_stretch))
        .add_child(Box::new(panel_tile))
        .add_child(Box::new(panel_cover));

    // ================= 3️⃣ TRANSPARENT OVERLAY =================
    // Красный фон + PNG с 30% прозрачности
    let overlay_30 = Panel::new(Box::new(Label::new("Overlay 30%").font_size(14.0).color(UColor::new(1.0, 1.0, 1.0, 1.0))))
        .background(UColor::new(0.8, 0.1, 0.1, 1.0))
        .background_texture_overlay(bg_id, BackgroundFit::Stretch, Some(UColor::new(1.0, 1.0, 1.0, 0.3)))
        .corner_radius(12.0).padding(EdgeInsets::all(16.0));

    // Зелёный фон + PNG с 70% прозрачности и Fit
    let overlay_70 = Panel::new(Box::new(Label::new("Overlay 70%").font_size(14.0).color(UColor::new(1.0, 1.0, 1.0, 1.0))))
        .background(UColor::new(0.1, 0.8, 0.1, 1.0))
        .background_texture_overlay(bg_id, BackgroundFit::Fit, Some(UColor::new(1.0, 1.0, 1.0, 0.7)))
        .corner_radius(12.0).padding(EdgeInsets::all(16.0));

    let sec3_row = Container::horizontal()
        .spacing(10.0)
        .alignment(Alignment::Center)
        .add_child(Box::new(overlay_30))
        .add_child(Box::new(overlay_70));

    // ================= 4️⃣ ICONBUTTONS =================
    let icon_plain = IconButton::new("Plain", icon_id, Size::new(20.0, 20.0))
        .padding(EdgeInsets::new(10.0, 16.0, 10.0, 16.0))
        .corner_radius(6.0)
        .color(UColor::new(0.2, 0.25, 0.35, 1.0))
        .on_click(|| println!("Plain icon clicked"));

    let icon_textured = IconButton::new("With Overlay", icon_id, Size::new(20.0, 20.0))
        .padding(EdgeInsets::new(10.0, 16.0, 10.0, 16.0))
        .corner_radius(6.0)
        .color(UColor::new(0.1, 0.1, 0.2, 1.0))
        .background_texture_overlay(bg_id, BackgroundFit::Stretch, Some(UColor::new(0.5, 0.8, 1.0, 0.4)))
        .on_click(|| println!("Textured icon clicked"));

    let sec4_row = Container::horizontal()
        .spacing(15.0)
        .alignment(Alignment::Center)
        .add_child(Box::new(icon_plain))
        .add_child(Box::new(icon_textured));

    // ================= 📦 FINAL ASSEMBLY =================
    Container::vertical()
        .alignment(Alignment::Center)
        .spacing(15.0)
        .padding(EdgeInsets::all(20.0))
        .add_child(Box::new(sec1_label))
        .add_child(Box::new(sec1_row))
        .add_child(Box::new(sec2_label))
        .add_child(Box::new(sec2_row))
        .add_child(Box::new(sec3_label))
        .add_child(Box::new(sec3_row))
        .add_child(Box::new(sec4_label))
        .add_child(Box::new(sec4_row))
}