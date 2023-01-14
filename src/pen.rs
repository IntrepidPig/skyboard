use linalg::*;

#[derive(Debug, Clone, Copy)]
pub struct PenEvent {
	pub pos: Point2,
	pub pressure: f32,
	pub speed: f32,
}

pub fn flat_pressure_curve(pressure: f32) -> f32 {
	pressure
}