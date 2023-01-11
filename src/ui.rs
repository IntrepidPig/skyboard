use wgpu::{Texture, TextureView, RenderPipeline, Instance, TextureFormat};
use winit::{window::{Window, WindowBuilder}, event_loop::EventLoop, dpi::LogicalSize};

use crate::{Graphics, blit::BlitPipeline};

pub struct Ui {
	pub graphics: Graphics,
	pub window: Window,
	surface: wgpu::Surface,
	blitter: BlitPipeline,
	pub canvas_view: CanvasView,
}

impl Ui {
	pub fn new(event_loop: &mut EventLoop<()>) -> anyhow::Result<Self> {
		let instance = Instance::new(wgpu::Backends::VULKAN);
		let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
			power_preference: wgpu::PowerPreference::HighPerformance,
			..Default::default()
		}))
			.ok_or(anyhow::format_err!("Failed to find a graphics adapter"))?;
		let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))?;
		let graphics = Graphics { instance, device, queue };
		let blitter = BlitPipeline::new(&graphics, TextureFormat::Bgra8UnormSrgb);
		
		let (width, height) = (1024, 768);
		let window = WindowBuilder::new()
			.with_inner_size(LogicalSize::new(width, height))
			.with_visible(true)
			.build(event_loop)?;
		
		let surface = unsafe { graphics.instance.create_surface(&window) };
		let config = wgpu::SurfaceConfiguration {
			usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::RENDER_ATTACHMENT,
			format: wgpu::TextureFormat::Bgra8UnormSrgb,
			width,
			height,
			present_mode: wgpu::PresentMode::AutoVsync,
			alpha_mode: wgpu::CompositeAlphaMode::Opaque,
		};
		surface.configure(&graphics.device, &config);
		
		let canvas_view = CanvasView::new(&graphics, width, height);
		
		Ok(Self {
			graphics,
			window,
			surface,
			blitter,
			canvas_view,
		})
	}
	
	pub fn handle_window_resize(&mut self) {
		let size = self.window.inner_size();
		let config = wgpu::SurfaceConfiguration {
			usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::RENDER_ATTACHMENT,
			format: wgpu::TextureFormat::Bgra8UnormSrgb,
			width: size.width,
			height: size.height,
			present_mode: wgpu::PresentMode::AutoVsync,
			alpha_mode: wgpu::CompositeAlphaMode::Opaque,
		};
		self.surface.configure(&self.graphics.device, &config);
		self.canvas_view = CanvasView::new(&self.graphics, size.width, size.height);
		
		self.window.request_redraw();
	}
	
	pub fn present(&mut self) -> anyhow::Result<()> {
		for _ in 0..3 {
			let surface_texture = match self.surface.get_current_texture() {
				Ok(surface_texture) => if surface_texture.suboptimal {
					drop(surface_texture);
					self.handle_window_resize();
					continue;
				} else {
					surface_texture
				},
				Err(wgpu::SurfaceError::Outdated) => {
					self.handle_window_resize();
					continue;
				},
				Err(e) => return Err(anyhow::Error::from(e)),
			};
			
			let surface_texture_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor {
				label: Some("Surface Texture View"),
				format: None,
				dimension: None,
				aspect: wgpu::TextureAspect::All,
				base_mip_level: 0,
				mip_level_count: None,
				base_array_layer: 0,
				array_layer_count: None,
			});
			self.blitter.blit(&self.graphics, self.canvas_view.get_texture_view(), &surface_texture_view);
			surface_texture.present();
			return Ok(())
		}
		
		Err(anyhow::format_err!("Failed to render to surface after 3 tries"))
	}
}

pub struct CanvasView {
	width: u32,
	height: u32,
	#[allow(unused)]
	target: Texture,
	target_view: TextureView,
}

impl CanvasView {
	pub fn new(graphics: &Graphics, width: u32, height: u32) -> Self {
		let (texture, texture_view) = Self::create_texture(graphics, width, height);
		Self {
			width,
			height,
			target: texture,
			target_view: texture_view,
		}
	}
	
	fn create_texture(graphics: &Graphics, width: u32, height: u32) -> (Texture, TextureView) {
		let texture = graphics.device.create_texture(&wgpu::TextureDescriptor {
			label: Some("Canvas View Target Texture"),
			size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: wgpu::TextureFormat::Rgba8Unorm,
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
		});
		let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
			label: Some("Canvas View Target TextureView"),
			format: Some(wgpu::TextureFormat::Rgba8Unorm),
			dimension: Some(wgpu::TextureViewDimension::D2),
			aspect: wgpu::TextureAspect::All,
			base_mip_level: 0,
			mip_level_count: None,
			base_array_layer: 0,
			array_layer_count: None,
		});
		(texture, texture_view)
	}
	
	pub fn get_texture_view(&self) -> &TextureView {
		&self.target_view
	}
	
	pub fn get_width(&self) -> u32 {
		self.width
	}
	
	pub fn get_height(&self) -> u32 {
		self.height
	}
}
