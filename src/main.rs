mod device;
mod bmfont;
mod texture_loader_adapter;

use anyhow::Result;
use wgpu_simple_ui::{DefaultPrimitives, UiRenderer, 
    common::types::{Alignment, BackgroundFit, EdgeInsets, Rect, Size, UColor}, 
    widgets::{canvas::CanvasItem, *}, *};
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
        let bg_id1 = ui_renderer.load_texture("assets/bg.png", &texture_loader).unwrap_or(0);
        let icon_id = ui_renderer.load_texture("assets/icon.png", &texture_loader).unwrap_or(0);

        // ===================================================================================
        // Строим дерево виджетов
        //let mut ui_widget = build_test_ui();
        // Строим дерево виджетов, передавая ID текстур
        let mut ui_widget = build_test_ui(bg_id1, icon_id);
        
 
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


/// Тестовый UI, демонстрирующий возможности нового универсального `Button`:
/// - Сплошной цвет фона
/// - Фоновое изображение с разными стратегиями `BackgroundFit` (Stretch, Tile, Cover)
/// - Кастомный фон через `CanvasItem` (градиент, рамка, линии)
/// - Произвольный layout внутри кнопки (иконка + текст вертикально/горизонтально)
/// - Скругление углов, обводка, отступы, margin
/// - Колбэк на клик
/// Тестовый UI, демонстрирующий возможности нового универсального `Button`
/// Тестовый UI, демонстрирующий возможности нового универсального `Button`
fn build_test_ui(bg_texture_id: u64, icon_texture_id: u64) -> impl Widget {
    use crate::common::types::{Rect, Line, UColor, EdgeInsets, Alignment, BackgroundFit, Size};
    use crate::ui::widgets::{Container, Label, Image, Button};

    // Белый цвет (замена UColor::white())
    let white = UColor::new(1.0, 1.0, 1.0, 1.0);

    // ========== 1. Твёрдый цвет ==========
    let solid_button = Button::new(
        Label::new("Solid Color")
            .font_size(18.0)
            .color(white)
    )
    .solid_color(UColor::new(0.2, 0.5, 0.8, 1.0))
    .corner_radius(10.0)
    .padding(EdgeInsets::all(16.0))
    .margin(EdgeInsets::all(8.0))
    .on_click(|| println!("✅ Solid button clicked"));

    // ========== 2. Фон: изображение (Stretch) ==========
    let stretch_button = Button::new(
        Label::new("Stretch")
            .font_size(16.0)
            .color(white)
    )
    .image(bg_texture_id, BackgroundFit::Stretch, white)
    .corner_radius(12.0)
    .padding(EdgeInsets::all(20.0))
    .on_click(|| println!("✅ Stretch button clicked"));

    // ========== 3. Фон: тайлинг (Tile) ==========
    let tile_button = Button::new(
        Label::new("Tile (scale=0.4)")
            .font_size(16.0)
            .color(UColor::new(1.0, 0.9, 0.2, 1.0))
    )
    .image(bg_texture_id, BackgroundFit::Tile { scale: 0.4 }, white)
    .corner_radius(8.0)
    .padding(EdgeInsets::all(16.0))
    .on_click(|| println!("✅ Tile button clicked"));

    // ========== 4. Фон: Cover ==========
    let cover_button = Button::new(
        Label::new("Cover")
            .font_size(16.0)
            .color(white)
    )
    .image(bg_texture_id, BackgroundFit::Cover, UColor::new(1.0, 0.8, 0.5, 0.9))
    .corner_radius(16.0)
    .padding(EdgeInsets::all(16.0))
    .on_click(|| println!("✅ Cover button clicked"));

    // ========== 5. Кастомный Canvas-фон ==========
    let canvas_width = 240.0;
    let canvas_height = 100.0;
    let canvas_items = vec![
        CanvasItem::Rect {
            rect: Rect::new(0.0, 0.0, canvas_width, canvas_height),
            color: UColor::new(0.1, 0.2, 0.35, 1.0),
        },
        CanvasItem::Rect {
            rect: Rect::new(0.0, 0.0, canvas_width, canvas_height * 0.5),
            color: UColor::new(0.3, 0.6, 0.9, 0.7),
        },
        CanvasItem::Line {
            line: Line::new(0.0, canvas_height * 0.25, canvas_width, canvas_height * 0.25, 1.5),
            color: UColor::new(1.0, 1.0, 1.0, 0.6),
        },
        CanvasItem::OutlineRect {
            rect: Rect::new(5.0, 5.0, canvas_width - 10.0, canvas_height - 10.0),
            radius: 8.0,
            thickness: 2.0,
            color: white,
        },
    ];

    let canvas_button = Button::new(
        Label::new("Canvas BG")
            .font_size(18.0)
            .color(white)
    )
    .canvas(canvas_items)
    .corner_radius(10.0)
    .padding(EdgeInsets::all(20.0))
    .on_click(|| println!("✅ Canvas button clicked"));

    // ========== 6. Кнопка с произвольным сложным содержимым ==========
    let complex_content = Container::vertical()
        .spacing(8.0)
        .alignment(Alignment::Center)
        .add_child(Box::new(Image::new(icon_texture_id, 48.0, 48.0)))
        .add_child(Box::new(
            Label::new("Awesome!")
                .font_size(20.0)
                .color(UColor::new(1.0, 0.9, 0.3, 1.0))
        ));

    let complex_button = Button::new(complex_content)
        .solid_color(UColor::new(0.4, 0.2, 0.6, 1.0))
        .corner_radius(16.0)
        .border(2.0, white)
        .padding(EdgeInsets::all(20.0))
        .on_click(|| println!("✅ Complex button clicked (icon + text)"));

    // ========== 7. Кнопка с обводкой и большим скруглением ==========
    let border_button = Button::new(
        Label::new("Bordered")
            .font_size(18.0)
            .color(white)
    )
    .solid_color(UColor::new(0.15, 0.2, 0.3, 1.0))
    .border(3.0, UColor::new(1.0, 0.5, 0.2, 1.0))
    .corner_radius(30.0)
    .padding(EdgeInsets::all(16.0));

    // ========== 8. Различные расположения иконки и текста внутри кнопки ==========
    let layout_label = Label::new("🎯 Icon + Text Layouts")
        .font_size(16.0)
        .color(UColor::new(0.8, 0.9, 1.0, 1.0))
        .margin(EdgeInsets::all(10.0));

    // 8.1 Иконка слева, текст справа (горизонтальный, по умолчанию)
    let left_icon = Container::horizontal()
        .spacing(10.0)
        .alignment(Alignment::Center)
        .add_child(Box::new(Image::new(icon_texture_id, 24.0, 24.0)))
        .add_child(Box::new(Label::new("← Icon").font_size(16.0).color(white)));

    let btn_left_icon = Button::new(left_icon)
        .solid_color(UColor::new(0.3, 0.4, 0.6, 1.0))
        .corner_radius(8.0)
        .padding(EdgeInsets::all(12.0))
        .on_click(|| println!("Left icon"));

    // 8.2 Текст слева, иконка справа
    let right_icon = Container::horizontal()
        .spacing(10.0)
        .alignment(Alignment::Center)
        .add_child(Box::new(Label::new("Icon →").font_size(16.0).color(white)))
        .add_child(Box::new(Image::new(icon_texture_id, 24.0, 24.0)));

    let btn_right_icon = Button::new(right_icon)
        .solid_color(UColor::new(0.3, 0.5, 0.7, 1.0))
        .corner_radius(8.0)
        .padding(EdgeInsets::all(12.0))
        .on_click(|| println!("Right icon"));

    // 8.3 Иконка сверху, текст снизу (вертикальный)
    let top_icon = Container::vertical()
        .spacing(8.0)
        .alignment(Alignment::Center)
        .add_child(Box::new(Image::new(icon_texture_id, 32.0, 32.0)))
        .add_child(Box::new(Label::new("Icon top").font_size(14.0).color(white)));

    let btn_top_icon = Button::new(top_icon)
        .solid_color(UColor::new(0.4, 0.5, 0.8, 1.0))
        .corner_radius(12.0)
        .padding(EdgeInsets::all(16.0))
        .on_click(|| println!("Top icon"));

    // 8.4 Текст сверху, иконка снизу (вертикальный)
    let bottom_icon = Container::vertical()
        .spacing(8.0)
        .alignment(Alignment::Center)
        .add_child(Box::new(Label::new("Icon bottom").font_size(14.0).color(white)))
        .add_child(Box::new(Image::new(icon_texture_id, 32.0, 32.0)));

    let btn_bottom_icon = Button::new(bottom_icon)
        .solid_color(UColor::new(0.2, 0.6, 0.5, 1.0))
        .corner_radius(12.0)
        .padding(EdgeInsets::all(16.0))
        .on_click(|| println!("Bottom icon"));

    // Горизонтальный ряд для компактного отображения
    let icon_layouts_row = Container::horizontal()
        .spacing(15.0)
        .alignment(Alignment::Center)
        .add_child(Box::new(btn_left_icon))
        .add_child(Box::new(btn_right_icon))
        .add_child(Box::new(btn_top_icon))
        .add_child(Box::new(btn_bottom_icon));
        
        //let image_button = build_image_button_example(bg_texture_id, icon_texture_id);
        //let tiled_button = build_tiled_image_button_example(bg_texture_id, icon_texture_id);

    // ========== 9. Различные расположения иконки и текста С ФОНОМ-ИЗОБРАЖЕНИЕМ ==========
    let border_color = UColor::new(1.0, 1.0, 1.0, 0.9);
    let bg_color = UColor::new(0.3, 0.3, 0.3, 0.9);

    let image_bg_label = Label::new("🖼️ With Image Background")
        .font_size(16.0)
        .color(UColor::new(0.8, 0.9, 1.0, 1.0))
        .margin(EdgeInsets::all(10.0));

    // 9.1 Иконка слева, текст справа
    let left_icon_img = Container::horizontal()
        .spacing(10.0)
        .alignment(Alignment::Center)
        .add_child(Box::new(Image::new(icon_texture_id, 24.0, 24.0)))
        .add_child(Box::new(Label::new("← Icon").font_size(16.0).color(white)));

    let btn_left_icon_img = Button::new(left_icon_img)
        .image(bg_texture_id, BackgroundFit::Stretch, white)
         .border(2.0, border_color)
        .corner_radius(8.0)
        .padding(EdgeInsets::all(12.0))
        .on_click(|| println!("Left icon with image bg"));

    // 9.2 Текст слева, иконка справа
    let right_icon_img = Container::horizontal()
        .spacing(10.0)
        .alignment(Alignment::Center)
        .add_child(Box::new(Label::new("Icon →").font_size(16.0).color(white)))
        .add_child(Box::new(Image::new(icon_texture_id, 24.0, 24.0)));

    let btn_right_icon_img = Button::new(right_icon_img)
        .image(bg_texture_id, BackgroundFit::Stretch, white)
         .border(2.0, border_color)
        .corner_radius(8.0)
        .padding(EdgeInsets::all(12.0))
        .on_click(|| println!("Right icon with image bg"));

    // 9.3 Иконка сверху, текст снизу
    let top_icon_img = Container::vertical()
        .spacing(8.0)
        .alignment(Alignment::Center)
        .add_child(Box::new(Image::new(icon_texture_id, 32.0, 32.0)))
        .add_child(Box::new(Label::new("Icon top").font_size(14.0).color(white)));

    let btn_top_icon_img = Button::new(top_icon_img)
        .image(bg_texture_id, BackgroundFit::Stretch, white)
        .corner_radius(12.0)
         .border(2.0, border_color)
        .padding(EdgeInsets::all(16.0))
        .on_click(|| println!("Top icon with image bg"));

    let black_color = UColor::new(0.0, 0.0, 0.0, 1.0);

    // 9.4 Текст сверху, иконка снизу
    let bottom_icon_img = Container::vertical()
        .spacing(8.0)
        .alignment(Alignment::Center)
        .add_child(Box::new(Label::new("Icon bottom").font_size(14.0).color(black_color)))
        .add_child(Box::new(Image::new(icon_texture_id, 32.0, 32.0)));

    let btn_bottom_icon_img = Button::new(bottom_icon_img)
        .image(bg_texture_id, BackgroundFit::Stretch, white)
        .border(2.0, border_color)
        .corner_radius(12.0)
        .padding(EdgeInsets::all(16.0))
        .on_click(|| println!("Bottom icon with image bg"));

    let combined_button = Button::new(Label::new("Combined"))
        .solid_color(UColor::new(0.2, 0.4, 0.6, 1.0))   // нижний слой
        .image(bg_texture_id, BackgroundFit::Cover, bg_color) // верхний слой
        .border(2.0, border_color)
        .corner_radius(12.0)
         .padding(EdgeInsets::all(16.0))
        .on_click(|| println!("Combined button clicked"));

    // Горизонтальный ряд для кнопок с фоновым изображением
    let icon_layouts_img_row = Container::horizontal()
        .spacing(15.0)
        .alignment(Alignment::Center)
        .add_child(Box::new(btn_left_icon_img))
        .add_child(Box::new(btn_right_icon_img))
        .add_child(Box::new(btn_top_icon_img))
        .add_child(Box::new(combined_button))
        .add_child(Box::new(btn_bottom_icon_img));
    
    // ========== 10. Комбинированный фон: цвет + полупрозрачное изображение + иконка и текст ==========
    let composite_label = Label::new("🎨 Composite: Color + PNG Overlay + Icon+Text")
        .font_size(16.0)
        .color(UColor::new(0.0, 0.0, 0.0, 1.0))
        .margin(EdgeInsets::all(10.0));

    // Содержимое: иконка + текст (горизонтально)
    let content = Container::horizontal()
        .spacing(12.0)
        .alignment(Alignment::Center)
        .add_child(Box::new(Image::new(icon_texture_id, 32.0, 32.0)))
        .add_child(Box::new(
            Label::new("Multi-layer")
                .font_size(18.0)
                .color(UColor::new(0.0, 0.0, 0.0, 1.0))
        ));

    // Кнопка:
    // 1. Сначала цвет (нижний слой)
    // 2. Поверх него полупрозрачное PNG (tint с альфой 0.7, чтобы просвечивал цвет)
    // 3. Затем содержимое (иконка+текст) будет поверх всего
    let composite_button = Button::new(content)
        .solid_color(UColor::new(0.2, 0.5, 0.8, 1.0))   // синий фон
        .image(bg_texture_id, BackgroundFit::Cover, UColor::new(1.0, 1.0, 1.0, 0.7)) // PNG с альфой 70%
        .border(2.0, UColor::new(1.0, 1.0, 1.0, 0.9))   // белая рамка
        .corner_radius(16.0)
        .padding(EdgeInsets::all(20.0))
        .on_click(|| println!("Composite button clicked"));

    // Альтернативный вариант: PNG поверх, но с другой стратегией заливки
    let composite_tile_button = Button::new(
        Container::horizontal()
            .spacing(12.0)
            .alignment(Alignment::Center)
            .add_child(Box::new(Image::new(icon_texture_id, 32.0, 32.0)))
            .add_child(Box::new(Label::new("Tile overlay").font_size(18.0).color(black_color)))
    )
    .solid_color(UColor::new(1.0,1.0, 1.0, 1.0))      //  фон
    .image(bg_texture_id, BackgroundFit::Tile { scale: 1.0 }, UColor::new(1.0, 1.0, 1.0, 0.5)) // тайлинг с полупрозрачностью
    .border(2.0, white)
    .corner_radius(16.0)
    .padding(EdgeInsets::all(20.0))
    .on_click(|| println!("Tile composite button clicked"));

    let composite_row = Container::horizontal()
        .spacing(20.0)
        .alignment(Alignment::Center)
        .add_child(Box::new(composite_button))
        .add_child(Box::new(composite_tile_button));

    // ========== Сборка в вертикальный контейнер ==========
    Container::vertical()
        .spacing(20.0)
        .alignment(Alignment::Center)
        .padding(EdgeInsets::all(30.0))
        //.add_child(Box::new(solid_button))
        //.add_child(Box::new(stretch_button))
        //.add_child(Box::new(tile_button))
        //.add_child(Box::new(cover_button))
        .add_child(Box::new(canvas_button))
        .add_child(Box::new(complex_button))
        .add_child(Box::new(border_button))
        //.add_child(Box::new(image_button))
        //.add_child(Box::new(tiled_button))
        .add_child(Box::new(layout_label))
        .add_child(Box::new(icon_layouts_row))
        .add_child(Box::new(image_bg_label))
        .add_child(Box::new(icon_layouts_img_row))
        .add_child(Box::new(composite_label))
        .add_child(Box::new(composite_row))
}


/// Пример: кнопка с фоновой текстурой, иконкой и текстом.
/// Внутри используется горизонтальный layout (иконка слева, текст справа).
fn build_image_button_example(bg_texture_id: u64, icon_texture_id: u64) -> impl Widget {
    use crate::ui::widgets::{Container, Image, Label, Button};
    use crate::common::types::{BackgroundFit, EdgeInsets, UColor, Size, Alignment};

    // 1. Создаём содержимое: горизонтальная строка (иконка + текст)
    let content = Container::horizontal()
        .spacing(12.0)                         // отступ между иконкой и текстом
        .alignment(Alignment::Center)          // выравнивание по вертикали
        .add_child(Box::new(Image::new(icon_texture_id, 32.0, 32.0)))
        .add_child(Box::new(
            Label::new("Image Button")
                .font_size(20.0)
                .color(UColor::new(1.0, 1.0, 1.0, 1.0))
        ));

    // 2. Оборачиваем в кнопку с фоновым изображением
    Button::new(content)
        .image(bg_texture_id, BackgroundFit::Stretch, UColor::new(1.0, 1.0, 1.0, 1.0))
        .corner_radius(12.0)
        .padding(EdgeInsets::all(16.0))
        .on_click(|| println!("Image button clicked"))
}

/// Пример: кнопка с тайловой текстурой фона, иконка сверху, текст снизу (вертикальный layout)
fn build_tiled_image_button_example(bg_texture_id: u64, icon_texture_id: u64) -> impl Widget {
    use crate::ui::widgets::{Container, Image, Label, Button};
    use crate::common::types::{BackgroundFit, EdgeInsets, UColor, Size, Alignment};

    let content = Container::vertical()
        .spacing(8.0)
        .alignment(Alignment::Center)
        .add_child(Box::new(Image::new(icon_texture_id, 40.0, 40.0)))
        .add_child(Box::new(
            Label::new("Tiled BG")
                .font_size(18.0)
                .color(UColor::new(1.0, 1.0, 0.8, 1.0))
        ));

    Button::new(content)
        .image(bg_texture_id, BackgroundFit::Tile { scale: 0.5 }, UColor::new(1.0, 1.0, 1.0, 0.9))
        .corner_radius(8.0)
        .padding(EdgeInsets::all(20.0))
        .border(2.0, UColor::new(1.0, 1.0, 1.0, 0.7))
        .on_click(|| println!("Tiled image button clicked"))
}