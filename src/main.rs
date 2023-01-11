#![allow(unused_imports)]

use blit::BlitPipeline;
use ui::{CanvasView, Ui};
use vello::kurbo::Affine;
use wgpu::{Instance, Queue, Device, RenderPipeline, TextureFormat};
use winit::{window::{WindowBuilder, Window}, event_loop::{EventLoop, ControlFlow}, dpi::{LogicalSize, PhysicalPosition}, event::{Event, WindowEvent, MouseButton, ElementState, TouchPhase, Touch}};
use winit::event::Tablet;
use linalg::*;

use canvas::*;

pub mod util;
pub mod canvas;
pub mod pen;
pub mod ui;
pub mod blit;

pub struct Graphics {
	instance: Instance,
	device: Device,
	queue: Queue,
}

struct App {
	ui: Ui,
	canvas: Canvas,
	current_pressure: f32,
}

impl App {
	pub fn new(event_loop: &mut EventLoop<()>) -> anyhow::Result<Self> {
		let ui = Ui::new(event_loop)?;
		let canvas = Canvas::new(&ui.graphics)?;
		
		Ok(Self {
			ui,
			canvas,
			current_pressure: 0.0,
		})
	}
	
	pub fn run(mut self, event_loop: EventLoop<()>) -> anyhow::Result<()> {
		self.canvas.render(&self.ui.graphics, &self.ui.canvas_view, Affine::IDENTITY);
		self.ui.present()?;
		
		event_loop.run(move |event, _target, control| {
			match self.handle_event(event, control) {
				Ok(()) => {},
				Err(e) => {
					log::error!("{e}");
					control.set_exit_with_code(1);
				}
			}
		});
	}
	
	fn handle_event(&mut self, event: Event<()>, control: &mut ControlFlow) -> anyhow::Result<()> {
		*control = ControlFlow::Wait;
			
		match event {
			Event::WindowEvent { window_id: _, event } => match event {
				WindowEvent::CloseRequested => *control = ControlFlow::Exit,
				WindowEvent::MouseInput { device_id: _, state, button, .. } => self.handle_mouse_button(state, button),
				WindowEvent::CursorMoved { device_id: _, position, .. } => self.handle_mouse_moved(position),
				WindowEvent::Touch(touch) => self.handle_touch(touch),
				WindowEvent::Tablet(tablet) => self.handle_tablet(tablet),
				WindowEvent::Resized(_size) => {
					self.ui.handle_window_resize();
				},
				_ => {},
			},
			Event::RedrawRequested(_window_id) => {
				self.canvas.render(&self.ui.graphics, &self.ui.canvas_view, Affine::IDENTITY);
				self.ui.present()?;
			},
			_ => {},
		};
		
		Ok(())
	}
	
	fn handle_mouse_button(&mut self, state: ElementState, button: MouseButton) {
		if button == MouseButton::Left {
			if state == ElementState::Pressed {
				self.canvas.start_stroke();
				self.ui.window.request_redraw();
			} else {
				self.canvas.end_stroke();
				self.ui.window.request_redraw();
			}
		}
	}
	
	fn handle_mouse_moved(&mut self, position: PhysicalPosition<f64>) {
		self.canvas.move_stroke(Vec2::new(position.x as f32, position.y as f32), 1.0);
		self.ui.window.request_redraw();
	}
	
	fn handle_touch(&mut self, _touch: Touch) {
		
	}
	
	fn handle_tablet(&mut self, tablet: Tablet) {
		match tablet {
			Tablet::Down => {
				self.canvas.start_stroke();
				self.ui.window.request_redraw();
			},
			Tablet::Up => {
				self.canvas.end_stroke();
				self.ui.window.request_redraw();
			},
			Tablet::Motion(pos) => {
				self.canvas.move_stroke(Vec2::new(pos.x as f32, pos.y as f32), self.current_pressure);
				self.ui.window.request_redraw();
			},
			Tablet::Pressure(pressure) => self.current_pressure = pressure.normalized() as f32,
			_ => {},
		}
	}
}

fn main() -> anyhow::Result<()> {
	env_logger::init();
	let mut event_loop = EventLoop::new();
	App::new(&mut event_loop).and_then(|app| app.run(event_loop))
}
