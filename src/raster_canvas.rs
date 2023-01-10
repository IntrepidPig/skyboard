use forma_render::{prelude::Point, PathBuilder, Order, gpu::Renderer, Composition, styling::Color, Path};
use linalg::Vec2;
use wgpu::{Texture, TextureView, TextureFormat};

use crate::{Graphics, pen::{PenEvent, flat_pressure_curve}};

pub struct Canvas {
	width: u32,
	height: u32,
	output: Texture,
	output_view: TextureView,
	strokes: Vec<Vec<PenEvent>>,
	rasters: Vec<Raster>,
	current_stroke: Option<StrokeInProgress>,
}

impl Canvas {
	pub fn new(graphics: &Graphics, width: u32, height: u32) -> anyhow::Result<Self> {
		let (output, output_view) = Self::create_texture(graphics, width, height);
		
		Ok(Self {
			width,
			height,
			output,
			output_view,
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
	
	pub fn start_stroke(&mut self) {
		let order = self.next_order();
		let layer = self.composition.create_layer();
		self.composition.insert(order, layer);
		self.current_stroke = Some(StrokeInProgress { events: Vec::new() })
	}
	
	pub fn move_stroke(&mut self, point: Vec2, pressure: f32) {
		let width = 4.0;
		
		if let Some(ref mut progress) = self.current_stroke {
			progress.move_to(point, pressure);
			/* if progress.events.len() >= 2 {
				let end = progress.events.len() - 1;
				if (progress.events[end].pos - progress.events[end - 1].pos).norm() < width * 2.0 {
					progress.events.pop();
					return;
				}
				let path = segment_to_path([progress.events[end - 1], progress.events[end]], width, &flat_pressure_curve);
				dbg!(&path);
				let layer = self.composition.get_mut(Order::new(self.next_order - 1).unwrap()).unwrap();
				layer.insert(&path);
			} */
		}
	}
	
	pub fn end_stroke(&mut self) {
		if let Some(progress) = self.current_stroke.take() {
			if progress.events.len() < 2 {
				return;
			}
			
			let path = pen_stroke_to_path(&progress.events, 4.0, &flat_pressure_curve);
			let layer = self.composition.get_mut(Order::new(self.next_order - 1).unwrap()).unwrap();
			layer.insert(&path);
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

fn segment_to_path(segment: [PenEvent; 2], width: f32, pressure_curve: &dyn Fn(f32) -> f32) -> Path {
	let side = width / 2.0;
	let extended = 0.0;
	
	let mut path_builder = PathBuilder::new();
	let a = segment[0].pos;
	let b = segment[1].pos;
	let dir = b - a;
	let perp = Vec2::new(dir.y, -dir.x).normalize();
	path_builder.move_to(vec_to_point(a - dir * extended - perp * side * pressure_curve(segment[0].pressure)));
	path_builder.line_to(vec_to_point(a - dir * extended + perp * side * pressure_curve(segment[0].pressure)));
	path_builder.line_to(vec_to_point(b + dir * extended + perp * side * pressure_curve(segment[1].pressure)));
	path_builder.line_to(vec_to_point(b + dir * extended - perp * side * pressure_curve(segment[1].pressure)));
	path_builder.build()
}

fn pen_stroke_to_path(stroke: &[PenEvent], width: f32, pressure_curve: &dyn Fn(f32) -> f32) -> Path {
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

struct StrokeData {
	// invariant: len >= 2
	points: Vec<StrokePoint>,
	segments: Vec<Segment>,
}

impl StrokeData {
	pub fn segment(&self, i: usize) -> [StrokePoint; 2] {
		[self.points[i], self.points[i + 1]]
	}
}

#[derive(Debug, Clone, Copy)]
struct StrokePoint {
	pos: Vec2,
	thickness: f32,
}

#[derive(Debug, Clone, Copy)]
struct Segment {
	// negative = left, positive = right
	bias: f32,
}

fn clean_stroke(stroke: StrokeData) {
	loop {
		// find the segment with two points closest together
		let dense_i = (0..stroke.points.len()).min_by_key(|&i| ((stroke.segment(i)[1].pos - stroke.segment(i)[0].pos).norm_squared() * 100000.0) as u64).unwrap();
		let dense_seg = stroke.segment(dense_i);
		let norm = (dense_seg[1].pos - dense_seg[0].pos).norm();
		if norm < dense_seg[0].thickness + dense_seg[1].thickness {
			break;
		}
	}
}

fn stroke_data_to_path(stroke_data: &StrokeData) {
	
}