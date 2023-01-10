use vello::peniko::{Stroke, Join, Cap, Color};
use vello::{Renderer, Scene, SceneBuilder, SceneFragment};
use vello::kurbo::{Line, Affine, Point};
use linalg::Vec2;
use wgpu::{Texture, TextureView, TextureFormat};

use crate::{Graphics, pen::{PenEvent, flat_pressure_curve}};

pub struct Canvas {
	width: u32,
	height: u32,
	output: Texture,
	output_view: TextureView,
	renderer: Renderer,
	layers: Vec<SceneFragment>,
	current_stroke: Option<StrokeInProgress>,
}

impl Canvas {
	pub fn new(graphics: &Graphics, width: u32, height: u32) -> anyhow::Result<Self> {
		let (output, output_view) = Self::create_texture(graphics, width, height);
		
		let renderer = Renderer::new(&graphics.device)
			.map_err(|e| anyhow::format_err!("{e}"))?;
		let scene = Scene::new();
		
		Ok(Self {
			width,
			height,
			output,
			output_view,
			renderer,
			layers: Vec::new(),
			current_stroke: None,
		})
	}
	
	fn create_texture(graphics: &Graphics, width: u32, height: u32) -> (Texture, TextureView) {
		let texture = graphics.device.create_texture(&wgpu::TextureDescriptor {
			label: Some("Canvas Output"),
			size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: wgpu::TextureFormat::Rgba8Unorm,
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
		});
		let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
			label: Some("Canvas Output View"),
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
	
	pub fn get_output(&self) -> &Texture {
		&self.output
	}
	
	pub fn get_output_view(&self) -> &wgpu::TextureView {
		&self.output_view
	}
	
	pub fn resize(&mut self, graphics: &Graphics, new_width: u32, new_height: u32) {
		let (new_texture, new_texture_view) = Self::create_texture(graphics, new_width, new_height);
		self.output = new_texture;
		self.output_view = new_texture_view;
		self.width = new_width;
		self.height = new_height;
		self.render(graphics);
	}
	
	pub fn render(&mut self, graphics: &Graphics) {
		let mut scene = Scene::new();
		let mut builder = SceneBuilder::for_scene(&mut scene);
		for layer in &self.layers {
			builder.append(layer, None);
		}
		builder.finish();
		
		self.renderer.render_to_texture(
			&graphics.device,
			&graphics.queue,
			&scene,
			&self.output_view,
			self.width,
			self.height,
		).unwrap();
	}
	
	pub fn start_stroke(&mut self) {
		self.current_stroke = Some(StrokeInProgress { events: Vec::new() })
	}
	
	pub fn move_stroke(&mut self, point: Vec2, pressure: f32) {
		if let Some(ref mut progress) = self.current_stroke {
			progress.move_to(point, pressure);
		}
	}
	
	pub fn end_stroke(&mut self) {
		if let Some(progress) = self.current_stroke.take() {
			if progress.events.len() < 2 {
				return;
			}
			
			let width = 4.0;
			let mut style = Stroke {
				width: 4.0,
				join: Join::Bevel,
				miter_limit: 1.0,
				start_cap: Cap::Round,
				end_cap: Cap::Round,
				dash_pattern: Default::default(),
				dash_offset: 0.0,
				scale: true,
			};
			let brush = Color { r: 255, g: 255, b: 255, a: 255 };
			
			let mut fragment = SceneFragment::new();
			let mut builder = SceneBuilder::for_fragment(&mut fragment);
			for i in 0..(progress.events.len() - 1) {
				let a = Point::new(progress.events[i].pos.x as f64, progress.events[i].pos.y as f64);
				let b = Point::new(progress.events[i+1].pos.x as f64, progress.events[i+1].pos.y as f64);
				style.width = width * (progress.events[i].pressure + progress.events[i+1].pressure) * 0.5;
				builder.stroke(&style, Affine::IDENTITY, brush, None, &Line::new(a, b));
			}
			builder.finish();
			self.layers.push(fragment);
		}
	}
	
	pub fn width(&self) -> u32 { self.width }
	
	pub fn height(&self) -> u32 { self.height }
}

struct StrokeInProgress {
	events: Vec<PenEvent>,
}

impl StrokeInProgress {
	pub fn new() -> Self {
		Self {
			events: Vec::new(),
		}
	}
	
	pub fn move_to(&mut self, point: Vec2, pressure: f32) {
		self.events.push(PenEvent {
			pos: point,
			pressure,
			speed: 1.0,
		});
	}
}
