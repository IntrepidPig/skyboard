use wgpu::{RenderPipeline, TextureFormat, TextureView, Device};

use crate::Graphics;

pub struct BlitPipeline {
	pipeline: RenderPipeline,
	sampler: wgpu::Sampler,
	bind_group_layout: wgpu::BindGroupLayout,
}

impl BlitPipeline {
	pub fn new(graphics: &Graphics, target_format: TextureFormat) -> Self {
		let shader = crate::util::load_wgsl_shader(&graphics.device, "blit.wgsl");
		let bind_group_layout = graphics.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: Some("Blit Bind Group Layout"),
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
			label: Some("Blit Pipeline Layout"),
			bind_group_layouts: &[
				&bind_group_layout,
			],
			push_constant_ranges: &[],
		});
		let pipeline = graphics.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Blit Render Pipeline"),
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
                    format: target_format,
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
			label: Some("Blit Sampler"),
			address_mode_u: wgpu::AddressMode::ClampToEdge,
			address_mode_v: wgpu::AddressMode::ClampToEdge,
			address_mode_w: wgpu::AddressMode::ClampToEdge,
			mag_filter: wgpu::FilterMode::Linear,
			min_filter: wgpu::FilterMode::Linear,
			mipmap_filter: wgpu::FilterMode::Linear,
			lod_min_clamp: 1.0,
			lod_max_clamp: 1.0,
			compare: None,
			anisotropy_clamp: None,
			border_color: None,
		});
		
		Self {
			pipeline,
			sampler,
			bind_group_layout,
		}
	}
	
	fn create_bind_group(device: &Device, layout: &wgpu::BindGroupLayout, view: &wgpu::TextureView, sampler: &wgpu::Sampler) -> wgpu::BindGroup {
		device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("Blit Bind Group"),
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
	
	pub fn blit(&self, graphics: &Graphics, source_view: &TextureView, target_view: &TextureView) {
		let bind_group = crate::util::time!("create bind group", Self::create_bind_group(&graphics.device, &self.bind_group_layout, &source_view, &self.sampler));
		let mut encoder = graphics.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Blit Command Encoder") });
		let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("Blit Render Pass"),
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: target_view,
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }),
					store: true,
				},
			})],
			depth_stencil_attachment: None,
		});
		render_pass.set_pipeline(&self.pipeline);
		render_pass.set_bind_group(0, &bind_group, &[]);
		render_pass.draw(0..3, 0..1);
		drop(render_pass);
		let commands = encoder.finish();
		graphics.queue.submit([commands]);
	}
}
