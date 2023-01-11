use vello::peniko::{Stroke, Join, Cap, Color, Fill, Brush};
use vello::{Renderer, Scene, FragmentBuilder, SceneFragment};
use vello::kurbo::{Line, Affine, Point, Rect};
use linalg::Vec2;
use wgpu::{TextureView};

use crate::pen::flat_pressure_curve;
use crate::ui::CanvasView;
use crate::{Graphics, pen::{PenEvent}};

pub struct Canvas {
	renderer: Renderer,
	layers: Vec<SceneFragment>,
	active_stroke: Option<ActiveStroke>,
}

impl Canvas {
	pub fn new(graphics: &Graphics) -> anyhow::Result<Self> {
		let renderer = Renderer::new(&graphics.device)
			.map_err(|e| anyhow::format_err!("{e}"))?;
			
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
			
		Ok(Self {
			renderer,
			layers,
			active_stroke: None,
		})
	}
	
	pub fn render(&mut self, graphics: &Graphics, view: &CanvasView, transform: Affine) {
		let mut scene = Scene::new();
		for layer in &self.layers {
			scene.append(layer, Some(transform));
		}
		
		self.renderer.render_to_texture(
			&graphics.device,
			&graphics.queue,
			&scene,
			view.get_texture_view(),
			view.get_width(),
			view.get_height(),
		).unwrap();
	}
	
	pub fn start_stroke(&mut self) {
		self.active_stroke = Some(ActiveStroke::new());
		self.layers.push(SceneFragment::new());
	}
	
	pub fn move_stroke(&mut self, point: Vec2, pressure: f32) {
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