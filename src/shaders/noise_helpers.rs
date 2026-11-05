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

/// Número pseudoaleatorio entre 0 y 1 para una celda.
pub fn cell_random(cell: (i32, i32, i32), salt: u32) -> f32 {
    let mut hash = (cell.0 as u32).wrapping_mul(73_856_093)
        ^ (cell.1 as u32).wrapping_mul(19_349_663)
        ^ (cell.2 as u32).wrapping_mul(83_492_791)
        ^ salt.wrapping_mul(2_654_435_761);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(1_274_126_177);
    hash ^= hash >> 16;
    hash as f32 / u32::MAX as f32
}
