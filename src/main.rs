use forma_render::{prelude::Point};
use wgpu::{Instance, Queue, Device};
use winit::{window::{WindowBuilder, Window}, event_loop::{EventLoop, ControlFlow}, dpi::LogicalSize, event::{Event, WindowEvent, MouseButton, ElementState}};

use canvas::*;

pub mod canvas;

pub struct Graphics {
	instance: Instance,
	device: Device,
	queue: Queue,
}

struct App {
	graphics: Graphics,
	window: Window,
	surface: wgpu::Surface,
	canvas: Canvas,
	state: State,
}

#[derive(Default)]
struct State {
	stroke_in_progress: Option<StrokeInProgress>,
}

impl App {
	pub fn new(event_loop: &mut EventLoop<()>) -> anyhow::Result<Self> {
		let (width, height) = (1024, 768);
		
		let instance = Instance::new(wgpu::Backends::PRIMARY);
		let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
			power_preference: wgpu::PowerPreference::HighPerformance,
			..Default::default()
		}))
			.ok_or(anyhow::format_err!("Failed to find a graphics adapter"))?;
		let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))?;
		let graphics = Graphics { instance, device, queue };
		
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
		
		let canvas = Canvas::new(&graphics, width, height)?;
		let state = State::default();
		
		Ok(Self {
			graphics,
			window,
			surface,
			canvas,
			state,
		})
	}
	
	pub fn run(mut self, event_loop: EventLoop<()>) -> anyhow::Result<()> {
		let mut stroke = self.canvas.start_stroke();
		stroke.move_to(Point::new(10.0, 10.0));
		stroke.move_to(Point::new(10.0, 20.0));
		stroke.move_to(Point::new(20.0, 20.0));
		self.canvas.end_stroke(stroke);
		self.canvas.render(&self.graphics);
		self.present_canvas()?;
		
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
				WindowEvent::MouseInput { device_id: _, state, button, .. } => {
					if button == MouseButton::Left {
						match state {
							ElementState::Pressed => {
								self.state.stroke_in_progress = Some(self.canvas.start_stroke());
							},
							ElementState::Released => {
								self.state.stroke_in_progress.take().map(|progress| self.canvas.end_stroke(progress));
								self.window.request_redraw();
							},
						}
					}
				},
				WindowEvent::CursorMoved { device_id: _, position, .. } => {
					self.state.stroke_in_progress
						.as_mut()
						.map(|progress| self.canvas.move_stroke(progress, Point::new(position.x as f32, position.y as f32)));
				},
				WindowEvent::Resized(_size) => {
					self.handle_window_resize();
				},
				_ => {},
			},
			Event::RedrawRequested(_window_id) => {
				self.canvas.render(&self.graphics);
				self.present_canvas()?;
			}
			_ => {},
		};
		
		Ok(())
	}
	
	fn handle_window_resize(&mut self) {
		log::debug!("Handling window resize");
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
		self.canvas.resize(&self.graphics, size.width, size.height);
		self.window.request_redraw();
	}
	
	fn present_canvas(&mut self) -> anyhow::Result<()> {
		for _ in 0..3 {
			let surface_texture = match self.surface.get_current_texture() {
				Ok(surface_texture) => if surface_texture.suboptimal {
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
			
			let mut encoder = self.graphics.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Copy To Surface") });
			encoder.copy_texture_to_texture(
				wgpu::ImageCopyTexture {
					texture: self.canvas.get_output(),
					mip_level: 0,
					origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
					aspect: wgpu::TextureAspect::All,
				},
				wgpu::ImageCopyTexture {
					texture: &surface_texture.texture,
					mip_level: 0,
					origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
					aspect: wgpu::TextureAspect::All,
				},
				wgpu::Extent3d { width: self.canvas.width(), height: self.canvas.height(), depth_or_array_layers: 1 }
			);
			let commands = encoder.finish();
			self.graphics.queue.submit([commands]);
			surface_texture.present();
			return Ok(())
		}
		
		Err(anyhow::format_err!("Failed to render to surface after 3 tries"))
	}
}

fn main() -> anyhow::Result<()> {
	env_logger::init();
	let mut event_loop = EventLoop::new();
	App::new(&mut event_loop).and_then(|app| app.run(event_loop))
}
