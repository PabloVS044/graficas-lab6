use noise::{NoiseFn, OpenSimplex};
use raylib::prelude::*;

/// Ruido OpenSimplex en un punto. Devuelve valores alrededor de 0.
pub fn sample(noise: &OpenSimplex, position: Vector3) -> f32 {
    noise.get([position.x as f64, position.y as f64, position.z as f64]) as f32
}

/// Suma varias octavas de ruido: cada una tiene el doble de frecuencia y la
/// mitad de amplitud. Más octavas dan más detalle, pero cuestan más.
/// Devuelve valores alrededor de 0.5.
pub fn fbm(noise: &OpenSimplex, mut position: Vector3, octaves: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut weight = 0.0;

    for _ in 0..octaves {
        value += sample(noise, position) * amplitude;
        weight += amplitude;
        position = position * 2.0;
        amplitude *= 0.5;
    }

    value / weight * 0.5 + 0.5
}
