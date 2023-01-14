use linalg::na::{Affine2, Translation2, Scale2};
use vello::peniko::{Stroke, Join, Cap, Color, Fill, Brush};
use vello::{Renderer, Scene, FragmentBuilder, SceneFragment};
use vello::kurbo::{Line, Affine, Point, Rect};
use linalg::prelude::*;
use wgpu::{TextureView, Texture};

use crate::pen::flat_pressure_curve;
use crate::util::*;
use crate::{Graphics, pen::{PenEvent}};

pub struct Canvas {
	layers: Vec<SceneFragment>,
	active_stroke: Option<ActiveStroke>,
}

impl Canvas {
	pub fn new() -> Self {
		let mut layers = Vec::new();
		
		// Add white background as first layer
		let mut builder = FragmentBuilder::new();
		builder.fill(
			Fill::NonZero,
			Affine::IDENTITY,
			&Brush::Solid(Color::rgb8(255, 255, 255)),
			None,
			&Rect { x0: -100000.0, y0: -100000.0, x1: 100000.0, y1: 100000.0 },
		);
		layers.push(builder.finish());
			
		Self {
			layers,
			active_stroke: None,
		}
	}
	
	pub fn start_stroke(&mut self) {
		self.active_stroke = Some(ActiveStroke::new());
		self.layers.push(SceneFragment::new());
	}
	
	pub fn move_stroke(&mut self, point: Point2, pressure: f32) {
		if let Some(ref mut active) = self.active_stroke {
			active.push_event(PenEvent {
				pos: point,
				pressure,
				speed: 1.0, // TODO
			});
			
			self.layers.pop();
			self.layers.push(active.get_fragment());
		}
	}
	
	pub fn end_stroke(&mut self) {
		self.active_stroke.take();
	}
}

pub struct ActiveStroke {
	events: Vec<PenEvent>,
	todo: usize,
	builder: FragmentBuilder,
}

impl ActiveStroke {
	pub fn new() -> Self {
		Self {
			events: Vec::new(),
			todo: 0,
			builder: FragmentBuilder::new(),
		}
	}
	
	pub fn push_event(&mut self, event: PenEvent) {
		let width = 8.0;
		
		let mut style = Stroke {
			width: 0.0,
			join: Join::Bevel,
			miter_limit: 1.0,
			start_cap: Cap::Round,
			end_cap: Cap::Round,
			dash_pattern: Default::default(),
			dash_offset: 0.0,
			scale: true,
		};
		let brush = Color { r: 0, g: 0, b: 0, a: 255 };
			
		self.events.push(event);
		for i in (self.todo+1)..self.events.len() {
			let a = Point::new(self.events[i-1].pos.x as f64, self.events[i-1].pos.y as f64);
			let b = Point::new(self.events[i].pos.x as f64, self.events[i].pos.y as f64);
			style.width = width * flat_pressure_curve((self.events[i-1].pressure + self.events[i].pressure) * 0.5);
			self.builder.stroke(&style, Affine::IDENTITY, brush, None, &Line::new(a, b));
		}
		self.todo = self.events.len() - 1;
	}
	
	pub fn get_fragment(&self) -> SceneFragment {
		self.builder.clone().finish()
	}
}

pub struct CanvasWidget {
	pub canvas: Canvas,
	width: u32,
	height: u32,
	/// The displacement of the view of the widget from the origin of the page
	/// Dragging left = panning right
	pub pan: Vec2,
	pub zoom: f64,
	renderer: Renderer,
	target: Texture,
	target_view: TextureView,
}

impl CanvasWidget {
	pub fn new(graphics: &Graphics, width: u32, height: u32) -> anyhow::Result<Self> {
		let renderer = Renderer::new(&graphics.device)
			.map_err(|e| anyhow::format_err!("{e}"))?;
		let canvas = Canvas::new();
		let (target, target_view) = Self::create_texture(graphics, width, height);
		
		Ok(Self {
			canvas,
			width,
			height,
			pan: Vec2::zero(),
			zoom: 1.0,
			renderer,
			target,
			target_view,
		})
	}
	
	pub fn resize(&mut self, graphics: &Graphics, new_width: u32, new_height: u32) {
		let (new_target, new_target_view) = Self::create_texture(graphics, new_width, new_height);
		self.target = new_target;
		self.target_view = new_target_view;
	}
	
	fn create_texture(graphics: &Graphics, width: u32, height: u32) -> (Texture, TextureView) {
		let texture = graphics.device.create_texture(&wgpu::TextureDescriptor {
			label: Some("Canvas Widget Target Texture"),
			size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: wgpu::TextureFormat::Rgba8Unorm,
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
		});
		let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
			label: Some("Canvas Widget Target TextureView"),
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
	
	/// Transform widget coordinates to page coordinates
	pub fn transform(&self) -> Affine2<f64> {
		Affine2::from_matrix_unchecked(
			Translation2::new(self.pan.x, self.pan.y).to_homogeneous()
				* Scale2::new(1.0 / self.zoom, 1.0 / self.zoom).to_homogeneous()
		)
	}
	
	/// Transform page coordinates to widget coordinates according to internal offset and zoom
	pub fn inv_transform(&self) -> Affine2<f64> {
		self.transform().inverse()
	}
	
	pub fn render(&mut self, graphics: &Graphics) {
		let mut scene = Scene::new();
		for layer in &self.canvas.layers {
			scene.append(layer, Some((self.inv_transform()).ltov()));
		}
		
		timeit!("render canvas", self.renderer.render_to_texture(
			&graphics.device,
			&graphics.queue,
			&scene,
			&self.target_view,
			self.width,
			self.height,
		).unwrap());
	}
}
