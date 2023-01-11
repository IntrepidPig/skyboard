use std::path::PathBuf;

use wgpu::Device;

pub fn load_wgsl_shader(device: &Device, filename: &str) -> wgpu::ShaderModule {
	let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	path.push("src/shaders");
	path.push(filename);
	let data = std::fs::read_to_string(&path).expect(&format!("Failed to read shader file at '{}'", path.display()));
	let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
		label: None,
		source: wgpu::ShaderSource::Wgsl(data.into()),
	});
	shader
}

macro_rules! time {
	($label:expr, $work:expr) => {
		{
			let start = std::time::Instant::now();
			let result = $work;
			let dur = start.elapsed();
			println!("TIME {:?}: {:.2}", $label, dur.as_secs_f64() * 1000.0);
			result
		}
	}
}

pub(crate) use time;