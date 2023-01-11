use linalg::Vec2;
use lyon::{lyon_tessellation::{StrokeTessellator, StrokeOptions, VertexBuffers, geometry_builder::simple_builder}, path::{LineCap, LineJoin, traits::{PathBuilder, Build}}, math::Point};
use wgpu::{Texture, TextureView, Buffer, RenderPipeline, util::DeviceExt};

use crate::{Graphics, pen::{PenEvent, flat_pressure_curve}};

#[derive(Debug)]
pub struct Layer {
	vertex: Buffer,
	index: Buffer,
	len: u32,
}

pub struct Canvas {
	width: u32,
	height: u32,
	output: Texture,
	output_view: TextureView,
	pipeline: RenderPipeline,
	todo_layers: Vec<VertexBuffers<Point, u16>>,
	layers: Vec<Layer>,
	current_stroke: Option<Vec<PenEvent>>,
}

impl Canvas {
	pub fn new(graphics: &Graphics, width: u32, height: u32) -> anyhow::Result<Self> {
		let (output, output_view) = Self::create_texture(graphics, width, height);
		let mut layers = Vec::new();
		
		let shader = graphics.device.create_shader_module(wgpu::include_wgsl!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/tri_canvas.wgsl")));
		let pipeline_layout = graphics.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("Tri Canvas Pipeline Layout"),
			bind_group_layouts: &[],
			push_constant_ranges: &[],
		});
		let pipeline = graphics.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Tri Canvas Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
					array_stride: std::mem::size_of::<Point>() as wgpu::BufferAddress,
					step_mode: wgpu::VertexStepMode::Vertex,
					attributes: &[
						wgpu::VertexAttribute {
							offset: 0,
							shader_location: 0,
							format: wgpu::VertexFormat::Float32x2,
						},
					],
				}],
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
		
		//let vertices = [Point::new(0.1, 0.1), Point::new(0.1, 0.5), Point::new(0.5, 0.5)];
		let vertices = [0.1f32, 0.1, 0.1, 0.5, 0.5, 0.5];
		let indices = [0u16, 1, 2];
		let vertex = graphics.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Vertex Buffer"),
			contents: unsafe { std::slice::from_raw_parts(vertices.as_ptr() as *const _, vertices.len() * std::mem::size_of::<f32>()) },
			usage: wgpu::BufferUsages::VERTEX,
		});
		let index = graphics.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Index Buffer"),
			contents: unsafe { std::slice::from_raw_parts(indices.as_ptr() as *const _, indices.len() * std::mem::size_of::<u16>()) },
			usage: wgpu::BufferUsages::INDEX,
		});
		layers.push(Layer {
			vertex,
			index,
			len: 3,
		});
		
		
		Ok(Self {
			width,
			height,
			output,
			output_view,
			pipeline,
			todo_layers: Vec::new(),
			layers,
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
		for todo_layer in std::mem::take(&mut self.todo_layers) {
			let vertex = graphics.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("Vertex Buffer"),
				contents: unsafe { std::slice::from_raw_parts(todo_layer.vertices.as_ptr() as *const _, todo_layer.vertices.len() * std::mem::size_of::<Point>()) },
				usage: wgpu::BufferUsages::VERTEX,
			});
			let index = graphics.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("Index Buffer"),
				contents: unsafe { std::slice::from_raw_parts(todo_layer.indices.as_ptr() as *const _, todo_layer.indices.len() * std::mem::size_of::<u16>()) },
				usage: wgpu::BufferUsages::INDEX,
			});
			self.layers.push(Layer {
				vertex,
				index,
				len: todo_layer.indices.len().try_into().unwrap(),
			})
		}
		
		let mut encoder = graphics.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Tri Canvas Render Encoder") });
		let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("Tri Canvas Render Pass"),
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: &self.output_view,
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }),
					store: true,
				},
			})],
			depth_stencil_attachment: None,
		});
		render_pass.set_pipeline(&self.pipeline);
		for layer in &self.layers {
			render_pass.set_vertex_buffer(0, layer.vertex.slice(..));
			render_pass.set_index_buffer(layer.index.slice(..), wgpu::IndexFormat::Uint16);
			render_pass.draw_indexed(0..layer.len, 0, 0..1);
		}
		drop(render_pass);
		let commands = encoder.finish();
		graphics.queue.submit([commands]);
	}
	
	pub fn start_stroke(&mut self) {
		self.current_stroke = Some(Vec::new());
	}
	
	pub fn move_stroke(&mut self, point: Vec2, pressure: f32) {
		if let Some(ref mut current_stroke) = self.current_stroke {
			current_stroke.push(PenEvent { pos: point, pressure, speed: 1.0 });
		}
	}
	
	pub fn end_stroke(&mut self) {
		if let Some(current_stroke) = self.current_stroke.take() {
			if current_stroke.len() < 1 {
				return;
			}
			
			let mut buffers = VertexBuffers::<Point, u16>::new();
			let mut vertex_builder = simple_builder(&mut buffers);
			let mut tesselator = StrokeTessellator::new();
			let mut options = StrokeOptions::default();
			options.start_cap = LineCap::Round;
			options.end_cap = LineCap::Round;
			options.line_join = LineJoin::Round;
			options.line_width = 4.0;
			options.variable_line_width = Some(0);
			options.miter_limit = StrokeOptions::DEFAULT_MITER_LIMIT;
			options.tolerance = 1.0;
			let mut builder = tesselator.builder_with_attributes(1, &options, &mut vertex_builder);
			
			builder.begin(Point::new(current_stroke[0].pos.x, current_stroke[0].pos.y), &[flat_pressure_curve(current_stroke[0].pressure)]);
			for &event in &current_stroke[1..] {
				builder.line_to(Point::new(event.pos.x, event.pos.y), &[flat_pressure_curve(event.pressure)]);
			}
			builder.build().unwrap();
			self.todo_layers.push(buffers);
		}
	}
	
	pub fn width(&self) -> u32 { self.width }
	
	pub fn height(&self) -> u32 { self.height }
}

