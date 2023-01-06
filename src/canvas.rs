use forma_render::{prelude::Point, PathBuilder, Order, gpu::Renderer, Composition, styling::Color, Path};
use linalg::Vec2;
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
		StrokeInProgress::new()
	}
	
	pub fn move_stroke(&mut self, progress: &mut StrokeInProgress, point: Vec2, pressure: f32) {
		progress.move_to(point, pressure);
	}
	
	pub fn end_stroke(&mut self, progress: StrokeInProgress) {
		if progress.events.len() < 2 {
			return;
		}
		let path = pen_stroke_to_path(&progress.events, &flat_pressure_curve);
		let mut layer = self.composition.create_layer();
		layer.insert(&path);
		let next_order = self.next_order();
		self.composition.insert(next_order, layer);
	}
	
	pub fn width(&self) -> u32 { self.width }
	
	pub fn height(&self) -> u32 { self.height }
}

pub struct StrokeInProgress {
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

pub struct PenEvent {
	pos: Vec2,
	pressure: f32,
	speed: f32,
}

fn pen_stroke_to_path(stroke: &[PenEvent], pressure_curve: &dyn Fn(f32) -> f32) -> Path {
	let width = 16.0;
	let side = width / 2.0;
	
	assert!(stroke.len() >= 2);
	let mut path_builder = PathBuilder::new();
	let a = stroke[0].pos;
	let b = stroke[1].pos;
	let dir = b - a;
	let perp = Vec2::new(dir.y, -dir.x).normalize();
	path_builder.move_to(vec_to_point(a - perp * side * pressure_curve(stroke[0].pressure)));
	path_builder.line_to(vec_to_point(a + perp * side));
	for i in 1..stroke.len() {
		let a = stroke[i - 1].pos;
		let b = stroke[i].pos;
		let dir = b - a;
		let perp = Vec2::new(dir.y, -dir.x).normalize();
		path_builder.line_to(vec_to_point(b - perp * side * pressure_curve(stroke[i].pressure)));
	}
	let a = stroke[stroke.len() - 1].pos;
	let b = stroke[stroke.len() - 2].pos;
	let dir = b - a;
	let perp = Vec2::new(dir.y, -dir.x).normalize();
	path_builder.line_to(vec_to_point(b - perp * side * pressure_curve(stroke[stroke.len() - 2].pressure)));
	for i in (0..(stroke.len() - 1)).rev() {
		let a = stroke[i + 1].pos;
		let b = stroke[i].pos;
		let dir = b - a;
		let perp = Vec2::new(dir.y, -dir.x).normalize();
		path_builder.line_to(vec_to_point(b - perp * side * pressure_curve(stroke[i].pressure)));
	}
	path_builder.build()
}

fn point_to_vec(point: Point) -> Vec2 {
	Vec2::new(point.x, point.y)
}

fn vec_to_point(vec: Vec2) -> Point {
	Point::new(vec.x, vec.y)
}

fn flat_pressure_curve(pressure: f32) -> f32 {
	pressure
}