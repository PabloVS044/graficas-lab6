//! Luna de la jungla: roca gris con mares oscuros y cráteres. Usa la misma esfera
//! que los demás cuerpos, dibujada más pequeña y en otra posición.

use noise::OpenSimplex;
use raylib::prelude::*;

use super::noise_helpers::{cell_random, fbm};
use super::{color, mix, smoothstep};
use crate::Uniforms;
use crate::fragment::Fragment;

/// Tamaño de los cráteres grandes: más alto da cráteres más pequeños y numerosos.
const LARGE_CRATER_GRID: f32 = 4.0;
/// Tamaño de los cráteres pequeños.
const SMALL_CRATER_GRID: f32 = 11.0;
/// Fracción de celdas de la cuadrícula que tienen un cráter.
const CRATER_CHANCE: f32 = 0.55;
/// Cuánto se oscurece el fondo y se aclara el borde de un cráter.
const CRATER_DEPTH: f32 = 0.42;
/// Cuánto oscurecen los mares: 0 no se ven.
const SEA_DARKNESS: f32 = 0.45;
/// Luz mínima del lado oscuro.
const AMBIENT: f32 = 0.05;

/// Cambio de brillo que causan los cráteres en un punto: negativo en el fondo y
/// positivo en el borde. Cada celda de una cuadrícula puede tener un cráter, y se
/// revisan las vecinas porque un cráter puede pasarse a la celda de al lado.
fn craters(position: Vector3, salt: u32) -> f32 {
    let (cx, cy, cz) = (
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    );
    let mut shade: f32 = 0.0;

    for x in cx - 1..=cx + 1 {
        for y in cy - 1..=cy + 1 {
            for z in cz - 1..=cz + 1 {
                let cell = (x, y, z);
                if cell_random(cell, salt) > CRATER_CHANCE {
                    continue;
                }

                let dx = x as f32 + cell_random(cell, salt + 1) - position.x;
                let dy = y as f32 + cell_random(cell, salt + 2) - position.y;
                let dz = z as f32 + cell_random(cell, salt + 3) - position.z;
                let radius = 0.22 + cell_random(cell, salt + 4) * 0.33;
                let distance = (dx * dx + dy * dy + dz * dz).sqrt() / radius;

                let floor = 1.0 - smoothstep(0.62, 0.86, distance);
                let rim = smoothstep(0.62, 0.92, distance) * (1.0 - smoothstep(0.92, 1.16, distance));
                let crater = rim * 0.65 - floor;
                if crater.abs() > shade.abs() {
                    shade = crater;
                }
            }
        }
    }

    shade
}

fn surface(noise: &OpenSimplex, p: Vector3) -> Vector3 {
    let dust = fbm(noise, p * 7.0 + color(30.0, 9.0, -14.0), 3);
    let rock = mix(color(0.34, 0.33, 0.31), color(0.62, 0.60, 0.56), smoothstep(0.32, 0.68, dust));

    let sea = smoothstep(0.50, 0.58, fbm(noise, p * 1.8 + color(-20.0, 6.0, 3.0), 3));
    let rock = rock * (1.0 - sea * SEA_DARKNESS);

    let shade = craters(p * LARGE_CRATER_GRID, 60) + craters(p * SMALL_CRATER_GRID, 70) * 0.6;
    rock * (1.0 + shade * CRATER_DEPTH)
}

pub fn shade(fragment: &Fragment, uniforms: &Uniforms) -> Vector3 {
    let p = fragment.object_position.normalize();
    surface(&uniforms.noise, p) * (AMBIENT + fragment.intensity * (1.0 - AMBIENT))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(i: usize) -> Vector3 {
        let angle = i as f32 * 2.4;
        let y = 1.0 - i as f32 / 1000.0;
        let r = (1.0 - y * y).max(0.0).sqrt();
        color(r * angle.cos(), y, r * angle.sin())
    }

    #[test]
    fn craters_have_dark_floors_and_bright_rims() {
        let values: Vec<f32> = (0..2000).map(|i| craters(point(i) * LARGE_CRATER_GRID, 60)).collect();
        assert!(values.iter().any(|&shade| shade < -0.5));
        assert!(values.iter().any(|&shade| shade > 0.3));
    }

    #[test]
    fn not_all_of_the_surface_is_a_crater() {
        let flat = (0..2000)
            .filter(|&i| craters(point(i) * LARGE_CRATER_GRID, 60).abs() < 0.01)
            .count();
        assert!(flat > 100, "solo {flat} puntos sin cráter");
    }
}
