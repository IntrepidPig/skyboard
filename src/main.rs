use wgpu::{Instance, Queue, Device, RenderPipeline, TextureFormat};
use winit::{window::{WindowBuilder, Window}, event_loop::{EventLoop, ControlFlow}, dpi::LogicalSize, event::{Event, WindowEvent, MouseButton, ElementState, TouchPhase, Tablet}};
use linalg::*;

use canvas::*;

pub mod util;
pub mod canvas;
pub mod tri_canvas;
//pub mod raster_canvas;
pub mod vello_canvas;
pub mod pen;

pub struct Graphics {
	instance: Instance,
	device: Device,
	queue: Queue,
}

struct App {
	graphics: Graphics,
	window: Window,
	surface: wgpu::Surface,
	pipeline: RenderPipeline,
	sampler: wgpu::Sampler,
	bind_group_layout: wgpu::BindGroupLayout,
	bind_group: wgpu::BindGroup,
	canvas: vello_canvas::Canvas,
	current_pressure: f32,
}

impl App {
	pub fn new(event_loop: &mut EventLoop<()>) -> anyhow::Result<Self> {
		let (width, height) = (1024, 768);
		
		let instance = Instance::new(wgpu::Backends::VULKAN);
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
		
		let shader = util::load_wgsl_shader(&graphics.device, "screen_tri.wgsl");
		let bind_group_layout = graphics.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: Some("Main Screen Bind Group Layout"),
			entries: &[
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Texture {
						sample_type: wgpu::TextureSampleType::Float { filterable: true },
						view_dimension: wgpu::TextureViewDimension::D2,
						multisampled: false,
					},
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
					binding: 1,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
					count: None,
				}
			],
		});
		
		let pipeline_layout = graphics.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("Main Screen Pipeline Layout"),
			bind_group_layouts: &[
				&bind_group_layout,
			],
			push_constant_ranges: &[],
		});
		let pipeline = graphics.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Main Screen Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
		
		let sampler = graphics.device.create_sampler(&wgpu::SamplerDescriptor {
			label: Some("Main Screen Output Sampler"),
			address_mode_u: wgpu::AddressMode::ClampToEdge,
			address_mode_v: wgpu::AddressMode::ClampToEdge,
			address_mode_w: wgpu::AddressMode::ClampToEdge,
			mag_filter: wgpu::FilterMode::Nearest,
			min_filter: wgpu::FilterMode::Nearest,
			mipmap_filter: wgpu::FilterMode::Linear,
			lod_min_clamp: 1.0,
			lod_max_clamp: 1.0,
			compare: None,
			anisotropy_clamp: None,
			border_color: None,
		});
		
		let forma_canvas = Canvas::new(&graphics, width, height)?;
		let tri_canvas = tri_canvas::Canvas::new(&graphics, width, height)?;
		let vello_canvas = vello_canvas::Canvas::new(&graphics, width, height)?;
		let canvas = vello_canvas;
		
		let bind_group = Self::create_bind_group(&graphics.device, &bind_group_layout, canvas.get_output_view(), &sampler);
		
		Ok(Self {
			graphics,
			window,
			surface,
			pipeline,
			sampler,
			bind_group_layout,
			bind_group,
			canvas,
			current_pressure: 0.0,
		})
	}
	
	pub fn run(mut self, event_loop: EventLoop<()>) -> anyhow::Result<()> {
		let mut stroke = self.canvas.start_stroke();
		self.canvas.move_stroke(Vec2::new(50.0, 50.0), 1.0);
		self.canvas.move_stroke(Vec2::new(250.0, 50.0), 1.0);
		self.canvas.move_stroke(Vec2::new(500.0, 50.0), 1.0);
		self.canvas.end_stroke();
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
				/* WindowEvent::MouseInput { device_id: _, state, button, .. } => {
					if button == MouseButton::Left {
						match state {
							ElementState::Pressed => {
								self.canvas.start_stroke();
								self.window.request_redraw();
							},
							ElementState::Released => {
								self.canvas.end_stroke();
								self.window.request_redraw();
							},
						}
					}
				},
				WindowEvent::CursorMoved { device_id: _, position, .. } => {
					self.canvas.move_stroke(Vec2::new(position.x as f32, position.y as f32), 1.0);
					self.window.request_redraw();
				},
				WindowEvent::Touch(touch) => {
					dbg!(touch);
					match touch.phase {
						TouchPhase::Started => {
							self.canvas.start_stroke();
						},
						TouchPhase::Moved => {
							self.canvas.move_stroke(Vec2::new(touch.location.x as f32, touch.location.y as f32), touch.force.map(|f| f.normalized() as f32).unwrap_or(1.0));
						},
						TouchPhase::Ended | TouchPhase::Cancelled => {
							self.canvas.end_stroke();
						}
					};
					self.window.request_redraw();
				}, */
				WindowEvent::Tablet(tablet) => match dbg!(tablet) {
					Tablet::Down => self.canvas.start_stroke(),
					Tablet::Up => {
						self.canvas.end_stroke();
						self.window.request_redraw();
					},
					Tablet::Motion(pos) => self.canvas.move_stroke(Vec2::new(pos.x as f32, pos.y as f32), self.current_pressure),
					Tablet::Pressure(pressure) => self.current_pressure = pressure.normalized() as f32,
					_ => {},
				}
				WindowEvent::Resized(_size) => {
					self.handle_window_resize();
				},
				_ => {},
			},
			Event::RedrawRequested(_window_id) => {
				self.canvas.render(&self.graphics);
				self.present_canvas()?;
			},
			_ => {},
		};
		
		Ok(())
	}
	
	fn create_bind_group(device: &Device, layout: &wgpu::BindGroupLayout, view: &wgpu::TextureView, sampler: &wgpu::Sampler) -> wgpu::BindGroup {
		device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("Main Screen Bind Group"),
			layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(view)
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Sampler(sampler),
				},
			],
		})
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
		self.bind_group = Self::create_bind_group(&self.graphics.device, &self.bind_group_layout, self.canvas.get_output_view(), &self.sampler);
		
		self.window.request_redraw();
	}
	
	fn present_canvas(&mut self) -> anyhow::Result<()> {
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
			
			let mut encoder = self.graphics.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Copy To Surface") });
			let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				label: Some("Main Screen Render Pass"),
				color_attachments: &[Some(wgpu::RenderPassColorAttachment {
					view: &surface_texture_view,
					resolve_target: None,
					ops: wgpu::Operations {
						load: wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }),
						store: true,
					},
				})],
				depth_stencil_attachment: None,
			});
			render_pass.set_pipeline(&self.pipeline);
			render_pass.set_bind_group(0, &self.bind_group, &[]);
			render_pass.draw(0..3, 0..1);
			drop(render_pass);
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
