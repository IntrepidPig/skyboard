use forma_render::{prelude::Point, PathBuilder, Order, gpu::Renderer, Composition, styling::Color};
use wgpu::{Texture, TextureView, TextureFormat};

use crate::Graphics;

pub struct Canvas {
	width: u32,
	height: u32,
	output: Texture,
	output_view: TextureView,
	renderer: Renderer,
	pub composition: Composition,
	next_order: u32,
}

impl Canvas {
	pub fn new(graphics: &Graphics, width: u32, height: u32) -> anyhow::Result<Self> {
		let (output, output_view) = Self::create_texture(graphics, width, height);
		
		let renderer = Renderer::new(&graphics.device, TextureFormat::Bgra8UnormSrgb, false);
		let composition = Composition::new();
		
		Ok(Self {
			width,
			height,
			output,
			output_view,
			renderer,
			composition,
			next_order: 1,
		})
	}
	
	fn create_texture(graphics: &Graphics, width: u32, height: u32) -> (Texture, TextureView) {
		let texture = graphics.device.create_texture(&wgpu::TextureDescriptor {
			label: Some("Canvas Output"),
			size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: wgpu::TextureFormat::Bgra8UnormSrgb,
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
		});
		let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
			label: Some("Canvas Output View"),
			format: Some(wgpu::TextureFormat::Bgra8UnormSrgb),
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
	
	pub fn resize(&mut self, graphics: &Graphics, new_width: u32, new_height: u32) {
		let (new_texture, new_texture_view) = Self::create_texture(graphics, new_width, new_height);
		self.output = new_texture;
		self.output_view = new_texture_view;
		self.width = new_width;
		self.height = new_height;
		self.render(graphics);
	}
	
	pub fn render(&mut self, graphics: &Graphics) {
		self.renderer.render_to_texture(
			&mut self.composition,
			&graphics.device,
			&graphics.queue,
			&self.output_view,
			self.width,
			self.height,
			Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 },
		);
	}
	
	pub fn next_order(&mut self) -> Order {
		let order = Order::new(self.next_order).unwrap();
		self.next_order += 1;
		order
	}
	
	pub fn start_stroke(&mut self) -> StrokeInProgress {
		StrokeInProgress { points: Vec::new() }
	}
	
	pub fn move_stroke(&mut self, progress: &mut StrokeInProgress, point: Point) {
		progress.points.push(point);
	}
	
	pub fn end_stroke(&mut self, progress: StrokeInProgress) {
		if progress.points.len() < 2 {
			return;
		}
		
		let mut path = PathBuilder::new();
		path.move_to(progress.points[0]);
		for point in &progress.points[1..] {
			path.line_to(*point);
		}
		let path = path.build();
		let mut layer = self.composition.create_layer();
		layer.insert(&path);
		let next_order = self.next_order();
		self.composition.insert(next_order, layer);
	}
	
	pub fn width(&self) -> u32 { self.width }
	
	pub fn height(&self) -> u32 { self.height }
}

pub struct StrokeInProgress {
	points: Vec<Point>,
}

impl StrokeInProgress {
	pub fn move_to(&mut self, point: Point) {
		self.points.push(point);
	}
}
