use std::path::PathBuf;

use vello::kurbo::{Point, Affine};
use wgpu::Device;
use linalg::{*, na::Affine2};

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

macro_rules! timeit {
	($label:expr, $work:expr) => {
		{
			let start = std::time::Instant::now();
			let result = $work;
			let dur = start.elapsed();
			println!("TIME {:?}: {:.2}ms", $label, dur.as_secs_f64() * 1000.0);
			result
		}
	}
}
pub(crate) use timeit;

pub trait VelloToLin<L> {
	fn vtol(self) -> L;
}

pub trait LinToVello<V> {
	fn ltov(self) -> V;
}

impl VelloToLin<Vec2> for Point {
    fn vtol(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
}

impl LinToVello<Point> for Vec2 {
    fn ltov(self) -> Point {
        Point::new(self.x, self.y)
    }
}

impl VelloToLin<Affine2<f64>> for Affine {
    fn vtol(self) -> Affine2<f64> {
		let vals = self.as_coeffs();
        Affine2::<f64>::from_matrix_unchecked(Mat3::new(
			vals[0],
			vals[1],
			vals[2],
			vals[3],
			vals[4],
			vals[5],
			0.0,
			0.0,
			1.0,
		))
    }
}

impl LinToVello<Affine> for Affine2<f64> {
    fn ltov(self) -> Affine {
		let mat = self.matrix();
        Affine::new([
			mat[(0, 0)],
			mat[(1, 0)],
			mat[(0, 1)],
			mat[(1, 1)],
			mat[(0, 2)],
			mat[(1, 2)],
		])
    }
}