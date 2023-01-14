#![allow(unused_imports)]

use blit::BlitPipeline;
use derive_more::From;
use ui::{Ui};
use util::{timeit, VelloToLin};
use vello::kurbo::{Affine, Point};
use wgpu::{Instance, Queue, Device, RenderPipeline, TextureFormat};
use winit::{window::{WindowBuilder, Window}, event_loop::{EventLoop, ControlFlow}, dpi::{LogicalSize, PhysicalPosition}, event::{Event, WindowEvent, MouseButton, ElementState, TouchPhase, Touch, MouseScrollDelta}};
use winit::event::Tablet;
use linalg::{prelude::*, na::{Affine2, Translation2, Isometry2}};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, From)]
enum Press {
	Mouse(MouseButton),
	Pen
}

struct InputState {
	mouse_press: Option<MouseButton>,
	pen_press: bool,
}

impl InputState {
	pub fn new() -> Self {
		Self {
			mouse_press: None,
			pen_press: false,
		}
	}
	
	pub fn handle_mouse_pressed(&mut self, button: MouseButton) -> bool {
		if self.mouse_press.is_none() {
			self.mouse_press = Some(button);
			true
		} else {
			false
		}
	}
	
	pub fn handle_mouse_released(&mut self, button: MouseButton) -> bool {
		if self.mouse_press == Some(button) {
			self.mouse_press = None;
			true
		} else {
			false
		}
	}
	
	pub fn is_mouse_dragging(&self, button: MouseButton) -> bool {
		self.mouse_press == Some(button)
	}
	
	pub fn handle_pen_pressed(&mut self) -> bool {
		!std::mem::replace(&mut self.pen_press, true)
	}
	
	pub fn handle_pen_released(&mut self) -> bool {
		!std::mem::replace(&mut self.pen_press, false)
	}
	
	pub fn is_pen_pressed(&self) -> bool {
		self.pen_press
	}
}

struct App {
	ui: Ui,
	old_mouse_pos: Point2,
	input_state: InputState,
	current_pressure: f32,
	offset: Vec2,
	zoom_level: i32,
}

impl App {
	pub fn new(event_loop: &mut EventLoop<()>) -> anyhow::Result<Self> {
		let ui = timeit!("ui init", Ui::new(event_loop)?);
		
		Ok(Self {
			ui,
			old_mouse_pos: Point2::new(0.0, 0.0),
			input_state: InputState::new(),
			current_pressure: 0.0,
			offset: Vec2::zero(),
			zoom_level: 0,
		})
	}
	
	pub fn run(mut self, event_loop: EventLoop<()>) -> anyhow::Result<()> {
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
				WindowEvent::CursorMoved { device_id: _, position, .. } => self.handle_mouse_moved(Point2::new(position.x, position.y)),
				WindowEvent::MouseWheel { device_id: _, delta, phase, .. } => self.handle_mouse_wheel(delta, phase),
				WindowEvent::Touch(touch) => self.handle_touch(touch),
				WindowEvent::Tablet(tablet) => self.handle_tablet(tablet),
				WindowEvent::Resized(_size) => {
					self.ui.handle_window_resize();
				},
				_ => {},
			},
			Event::RedrawRequested(_window_id) => {
				self.ui.canvas.render(&self.ui.graphics);
				self.ui.present()?;
			},
			_ => {},
		};
		
		Ok(())
	}
	
	fn handle_mouse_button(&mut self, state: ElementState, button: MouseButton) {
		if state == ElementState::Pressed {
			let drag_start = self.input_state.handle_mouse_pressed(button);
			
			if button == MouseButton::Left && drag_start {
				self.ui.canvas.canvas.start_stroke();
				self.ui.window.request_redraw();
			}
		} else if state == ElementState::Released {
			let drag_end = self.input_state.handle_mouse_released(button);
			
			if button == MouseButton::Left && drag_end {
				self.ui.canvas.canvas.end_stroke();
				self.ui.window.request_redraw();
			}
		}
	}
	
	fn handle_mouse_moved(&mut self, position: Point2) {
		let widget_delta = position - self.old_mouse_pos;
		let delta = self.ui.canvas.transform() * widget_delta;
		
		if self.input_state.is_mouse_dragging(MouseButton::Left) {
			self.ui.canvas.canvas.move_stroke(self.ui.canvas.transform() * position, 1.0);
			self.ui.window.request_redraw();
		} else if self.input_state.is_mouse_dragging(MouseButton::Middle) {
			self.ui.canvas.pan -= delta;
			self.ui.window.request_redraw();
		}
		
		self.old_mouse_pos = position;
	}
	
	fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta, _phase: TouchPhase) {
		match delta {
			MouseScrollDelta::LineDelta(_h, v) => {
				if v > 0.0 {
					self.zoom_level += 1;
				} else if v < 0.0 {
					self.zoom_level -= 1;
				}
				self.ui.canvas.zoom = 2.0f64.powi(self.zoom_level);
				
				self.ui.window.request_redraw();
			},
			MouseScrollDelta::PixelDelta(_) => todo!(),
		}
	}
	
	fn handle_touch(&mut self, _touch: Touch) {
		
	}
	
	fn handle_tablet(&mut self, tablet: Tablet) {
		match tablet {
			Tablet::Down => {
				self.input_state.handle_pen_pressed();
				
				self.ui.canvas.canvas.start_stroke();
				self.ui.window.request_redraw();
			},
			Tablet::Up => {
				self.input_state.handle_pen_released();
				
				self.ui.canvas.canvas.end_stroke();
				self.ui.window.request_redraw();
			},
			Tablet::Motion(pos) => {
				if self.input_state.is_pen_pressed() {
					self.ui.canvas.canvas.move_stroke(self.ui.canvas.transform() * Point2::new(pos.x, pos.y), self.current_pressure);
					self.ui.window.request_redraw();
				}
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
