//! Fondo procedural: nebulosas muy oscuras y pocas estrellas. Depende de hacia
//! dónde mira cada píxel, así que se mueve al orbitar y parece estar lejos.
//!
//! Las nebulosas son caras y casi no tienen detalle fino, así que se calculan
//! una sola vez en una tabla generada con ruido (no es una imagen externa).

use noise::OpenSimplex;
use rayon::prelude::*;
use raylib::prelude::*;

use crate::Uniforms;
use crate::framebuffer::Framebuffer;
use crate::shaders::noise_helpers::fbm;
use crate::shaders::smoothstep;

const TABLE_WIDTH: usize = 1024;
const TABLE_HEIGHT: usize = 512;
/// Más alto da estrellas más pequeñas y numerosas.
const STAR_GRID: f32 = 90.0;
/// Fracción de celdas de la cuadrícula que contienen una estrella.
const STAR_CHANCE: f32 = 0.06;

pub struct Background {
    nebula: Vec<Vector3>,
}

impl Background {
    pub fn new(noise: &OpenSimplex) -> Self {
        let nebula = (0..TABLE_WIDTH * TABLE_HEIGHT)
            .into_par_iter()
            .map(|index| {
                let u = (index % TABLE_WIDTH) as f32 / TABLE_WIDTH as f32;
                let v = (index / TABLE_WIDTH) as f32 / (TABLE_HEIGHT - 1) as f32;
                nebula_color(noise, direction_from_table(u, v))
            })
            .collect();

        Background { nebula }
    }

    /// Pinta el fondo completo. Reemplaza a limpiar el framebuffer.
    pub fn paint(&self, framebuffer: &mut Framebuffer, uniforms: &Uniforms) {
        let width = framebuffer.width;

        framebuffer
            .colors
            .par_chunks_exact_mut(width)
            .enumerate()
            .for_each(|(y, row)| {
                for (x, pixel) in row.iter_mut().enumerate() {
                    let direction = uniforms.rays.direction(x, y);
                    *pixel = self.nebula_at(direction) + star_color(direction);
                }
            });

        framebuffer.depths.fill(f32::INFINITY);
    }

    /// Lee la tabla interpolando entre las cuatro celdas más cercanas.
    fn nebula_at(&self, d: Vector3) -> Vector3 {
        let u = d.z.atan2(d.x) / std::f32::consts::TAU + 0.5;
        let v = d.y.clamp(-1.0, 1.0).acos() / std::f32::consts::PI;

        let tx = u * TABLE_WIDTH as f32;
        let ty = v * (TABLE_HEIGHT - 1) as f32;
        let x0 = tx.floor() as usize % TABLE_WIDTH;
        let x1 = (x0 + 1) % TABLE_WIDTH;
        let y0 = (ty.floor() as usize).min(TABLE_HEIGHT - 1);
        let y1 = (y0 + 1).min(TABLE_HEIGHT - 1);
        let (fx, fy) = (tx.fract(), ty.fract());

        let at = |x: usize, y: usize| self.nebula[y * TABLE_WIDTH + x];
        let top = at(x0, y0) * (1.0 - fx) + at(x1, y0) * fx;
        let bottom = at(x0, y1) * (1.0 - fx) + at(x1, y1) * fx;
        top * (1.0 - fy) + bottom * fy
    }
}

/// Dirección que corresponde a una celda de la tabla (`u`, `v` entre 0 y 1).
fn direction_from_table(u: f32, v: f32) -> Vector3 {
    let longitude = (u - 0.5) * std::f32::consts::TAU;
    let latitude = v * std::f32::consts::PI;
    Vector3::new(
        latitude.sin() * longitude.cos(),
        latitude.cos(),
        latitude.sin() * longitude.sin(),
    )
}

/// Casi negro, con velos de vino y de verde azulado frío. Las franjas de polvo
/// apagan parte de ambos para dar sensación de profundidad.
fn nebula_color(noise: &OpenSimplex, d: Vector3) -> Vector3 {
    let void = Vector3::new(0.006, 0.006, 0.014);
    let wine = Vector3::new(0.30, 0.025, 0.07);
    let teal = Vector3::new(0.015, 0.13, 0.15);

    let warm = fbm(noise, d * 1.7 + Vector3::new(3.0, 1.0, 8.0), 5);
    let cold = fbm(noise, d * 2.2 + Vector3::new(-6.0, 9.0, 2.0), 5);
    let dust = fbm(noise, d * 4.5 + Vector3::new(warm, cold, 0.0) * 2.5, 5);

    let veil = smoothstep(0.30, 0.62, dust);
    void + wine * smoothstep(0.44, 0.68, warm) * veil + teal * smoothstep(0.48, 0.70, cold) * veil
}

/// Divide el espacio en una cuadrícula; algunas celdas tienen una estrella en
/// una posición al azar. El azar sale de un hash, así no cambia entre cuadros.
fn star_color(direction: Vector3) -> Vector3 {
    let cell = direction * STAR_GRID;
    let (cx, cy, cz) = (cell.x.floor(), cell.y.floor(), cell.z.floor());
    let seed = (cx as i32, cy as i32, cz as i32);

    if random(seed, 0) > STAR_CHANCE {
        return Vector3::zero();
    }

    let dx = cell.x - cx - (0.2 + random(seed, 1) * 0.6);
    let dy = cell.y - cy - (0.2 + random(seed, 2) * 0.6);
    let dz = cell.z - cz - (0.2 + random(seed, 3) * 0.6);
    let distance = (dx * dx + dy * dy + dz * dz).sqrt();

    let brightness = random(seed, 4).powf(3.0);
    let glow = 1.0 - smoothstep(0.03, 0.09 + brightness * 0.08, distance);

    // Unas pocas tiran a rojo y otras a azul.
    let tint = if random(seed, 5) > 0.5 {
        Vector3::new(1.0, 0.72, 0.68)
    } else {
        Vector3::new(0.72, 0.82, 1.0)
    };
    tint * glow * (0.18 + brightness * 0.75)
}

/// Número pseudoaleatorio entre 0 y 1 para una celda.
fn random(cell: (i32, i32, i32), salt: u32) -> f32 {
    let mut hash = (cell.0 as u32).wrapping_mul(73_856_093)
        ^ (cell.1 as u32).wrapping_mul(19_349_663)
        ^ (cell.2 as u32).wrapping_mul(83_492_791)
        ^ salt.wrapping_mul(2_654_435_761);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(1_274_126_177);
    hash ^= hash >> 16;
    hash as f32 / u32::MAX as f32
}
