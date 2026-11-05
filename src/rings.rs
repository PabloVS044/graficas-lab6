//! Anillos de energía del sol. No agregan geometría: para cada píxel se calcula
//! dónde cruza el rayo de la cámara el plano de cada anillo. Si ese punto cae a
//! la distancia correcta del centro, el píxel recibe la luz del anillo.
//!
//! Cada anillo nace dentro del sol, se expande hacia afuera y se apaga.

use rayon::prelude::*;
use raylib::prelude::*;

use crate::Uniforms;
use crate::framebuffer::Framebuffer;
use crate::shaders::noise_helpers::cell_random;
use crate::shaders::{color, mix, smoothstep};

/// Cantidad de anillos: más da un sol más cargado y un poco más lento.
const RING_COUNT: usize = 7;
/// Radio con el que nace un anillo, todavía oculto dentro del sol.
const START_RADIUS: f32 = 0.80;
/// Radio con el que termina de apagarse.
const END_RADIUS: f32 = 1.1;
/// Ancho de un anillo.
const RING_WIDTH: f32 = 0.2;
/// Vueltas por segundo del ciclo de nacer y apagarse.
const EXPANSION_SPEED: f32 = 0.055;
/// Anillos delgados que llegan más lejos. Se suman a los gruesos.
const THIN_RING_COUNT: usize = 5;
/// Radio con el que terminan de apagarse los delgados.
const THIN_END_RADIUS: f32 = 1.45;
/// Ancho de los delgados.
const THIN_RING_WIDTH: f32 = 0.03;
/// Velocidad de los delgados: al recorrer más distancia van un poco más lento.
const THIN_EXPANSION_SPEED: f32 = 0.04;
/// Brillo general: 0 apaga los anillos.
const RING_BRIGHTNESS: f32 = 2.3;
/// Brillo de los anillos que pasan por delante del disco, para que no se vean
/// como rayas sobre la superficie: 1 los deja igual que los de afuera.
const OVER_SUN_BRIGHTNESS: f32 = 0.3;
/// Radio aproximado de la esfera. Sirve para que los anillos se oculten detrás.
const SUN_RADIUS: f32 = 0.85;

/// Dirección fija de un anillo y lo que lo distingue de los demás.
struct Ring {
    normal: Vector3,
    tangent: Vector3,
    bitangent: Vector3,
    phase: f32,
    end_radius: f32,
    width: f32,
    speed: f32,
}

/// Estado de un anillo en este cuadro.
struct ActiveRing<'a> {
    ring: &'a Ring,
    /// Radio del borde exterior.
    radius: f32,
    width: f32,
    /// Se apaga al final de su expansión.
    fade: f32,
}

pub struct Rings {
    rings: Vec<Ring>,
}

impl Rings {
    pub fn new() -> Self {
        let mut rings = ring_family(RING_COUNT, 0, 0.0, END_RADIUS, RING_WIDTH, EXPANSION_SPEED);
        // Se giran las direcciones para que no coincidan con las de los gruesos.
        rings.extend(ring_family(
            THIN_RING_COUNT,
            100,
            1.3,
            THIN_END_RADIUS,
            THIN_RING_WIDTH,
            THIN_EXPANSION_SPEED,
        ));

        Rings { rings }
    }

    /// Suma la luz de los anillos a lo que ya está pintado.
    pub fn paint(&self, framebuffer: &mut Framebuffer, uniforms: &Uniforms) {
        let active: Vec<ActiveRing> = self
            .rings
            .iter()
            .filter_map(|ring| activate(ring, uniforms.time))
            .collect();

        let width = framebuffer.width;
        let depths = &framebuffer.depths;
        let origin = uniforms.camera_position;

        framebuffer
            .colors
            .par_chunks_exact_mut(width)
            .enumerate()
            .for_each(|(y, row)| {
                for (x, pixel) in row.iter_mut().enumerate() {
                    let direction = uniforms.rays.direction(x, y);
                    let covers_sun = depths[y * width + x].is_finite();
                    *pixel += rays_light(&active, origin, direction, covers_sun, uniforms.time);
                }
            });
    }
}

/// Crea un grupo de anillos iguales. Reparte sus normales sobre una esfera
/// (espiral de Fibonacci), para que rodeen al sol en todas las direcciones.
fn ring_family(
    count: usize,
    seed: i32,
    rotation: f32,
    end_radius: f32,
    width: f32,
    speed: f32,
) -> Vec<Ring> {
    let golden_angle = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());

    (0..count)
        .map(|index| {
            let i = index as f32;
            let y = 1.0 - (i + 0.5) * 2.0 / count as f32;
            let r = (1.0 - y * y).sqrt();
            let angle = i * golden_angle + rotation;
            let normal = color(r * angle.cos(), y, r * angle.sin());

            let helper = if normal.y.abs() < 0.9 {
                color(0.0, 1.0, 0.0)
            } else {
                color(1.0, 0.0, 0.0)
            };
            let tangent = normal.cross(helper).normalize();

            Ring {
                normal,
                tangent,
                bitangent: normal.cross(tangent),
                phase: cell_random((seed + index as i32, 3, 9), 0),
                end_radius,
                width,
                speed,
            }
        })
        .collect()
}

/// Avance del anillo en su ciclo. Solo está activo mientras se ve.
fn activate(ring: &Ring, time: f32) -> Option<ActiveRing<'_>> {
    let age = (time * ring.speed + ring.phase).fract();
    let fade = (1.0 - smoothstep(0.7, 1.0, age)) * smoothstep(0.0, 0.08, age);
    if fade <= 0.0 {
        return None;
    }

    Some(ActiveRing {
        ring,
        radius: mix_scalar(START_RADIUS, ring.end_radius, age),
        width: ring.width,
        fade,
    })
}

fn mix_scalar(a: f32, b: f32, amount: f32) -> f32 {
    a * (1.0 - amount) + b * amount
}

/// Distancia desde la cámara a la que el rayo toca el sol.
fn distance_to_sun(origin: Vector3, direction: Vector3) -> f32 {
    let along = -origin.dot(direction);
    let closest = origin.length_sqr() - along * along;
    along - (SUN_RADIUS * SUN_RADIUS - closest).max(0.0).sqrt()
}

/// Luz total que recibe un rayo sumando todos los anillos.
fn rays_light(
    active: &[ActiveRing],
    origin: Vector3,
    direction: Vector3,
    covers_sun: bool,
    time: f32,
) -> Vector3 {
    // Si el rayo pasa lejos de todos los anillos no hace falta revisarlos.
    let along = -(origin.x * direction.x + origin.y * direction.y + origin.z * direction.z);
    let origin_sqr = origin.x * origin.x + origin.y * origin.y + origin.z * origin.z;
    let reach = END_RADIUS.max(THIN_END_RADIUS);
    if origin_sqr - along * along > reach * reach {
        return Vector3::zero();
    }

    let limit = if covers_sun {
        distance_to_sun(origin, direction)
    } else {
        f32::INFINITY
    };

    let mut light = Vector3::zero();
    for ring in active {
        if let Some(value) = ring_light(ring, origin, direction, limit, time) {
            light += value;
        }
    }
    let over_sun = if covers_sun { OVER_SUN_BRIGHTNESS } else { 1.0 };
    light * RING_BRIGHTNESS * over_sun
}

/// Luz de un anillo para un rayo, si lo toca antes de llegar a `limit`.
fn ring_light(
    active: &ActiveRing,
    origin: Vector3,
    direction: Vector3,
    limit: f32,
    time: f32,
) -> Option<Vector3> {
    let n = active.ring.normal;

    // Cruce del rayo con el plano del anillo, que pasa por el centro del sol.
    let denominator = direction.x * n.x + direction.y * n.y + direction.z * n.z;
    if denominator.abs() < 1e-4 {
        return None;
    }
    let t = -(origin.x * n.x + origin.y * n.y + origin.z * n.z) / denominator;
    if t <= 0.0 || t > limit {
        return None;
    }

    let hit = (
        origin.x + direction.x * t,
        origin.y + direction.y * t,
        origin.z + direction.z * t,
    );
    let distance = (hit.0 * hit.0 + hit.1 * hit.1 + hit.2 * hit.2).sqrt();

    // 0 en el borde interior del anillo y 1 en el exterior.
    let across = (distance - (active.radius - active.width)) / active.width;
    if !(0.0..=1.0).contains(&across) {
        return None;
    }

    let hit = color(hit.0, hit.1, hit.2);
    let angle = hit.dot(active.ring.bitangent).atan2(hit.dot(active.ring.tangent));

    // Un anillo visto casi de canto queda más delgado que un píxel y se vería
    // punteado, así que se desvanece.
    let edge_on = smoothstep(0.03, 0.14, denominator.abs());

    Some(band_color(across, angle, active.ring.phase, time) * active.fade * edge_on)
}

/// Aspecto de la banda: un borde exterior encendido y franjas rojas adentro,
/// con pulsos de energía que recorren el anillo.
fn band_color(across: f32, angle: f32, phase: f32, time: f32) -> Vector3 {
    let edge = smoothstep(0.72, 0.92, across) * (1.0 - smoothstep(0.92, 1.0, across));
    let stripes = ((across * 3.0 * std::f32::consts::TAU).sin() * 0.5 + 0.5).powi(2)
        * smoothstep(0.0, 0.2, across);

    let turn = phase * std::f32::consts::TAU;
    let pulse = 0.55 + 0.45 * (angle * 3.0 + turn + time * (0.5 + phase)).sin();

    let hot = color(1.0, 0.11, 0.03);
    let deep_red = color(0.85, 0.01, 0.005);
    mix(deep_red * stripes * 0.8, hot, edge) * pulse
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_ring() -> Ring {
        Ring {
            normal: color(0.0, 1.0, 0.0),
            tangent: color(1.0, 0.0, 0.0),
            bitangent: color(0.0, 0.0, 1.0),
            phase: 0.0,
            end_radius: END_RADIUS,
            width: RING_WIDTH,
            speed: EXPANSION_SPEED,
        }
    }

    #[test]
    fn ray_only_lights_up_at_the_ring_radius() {
        let ring = flat_ring();
        let active = ActiveRing { ring: &ring, radius: 1.5, width: RING_WIDTH, fade: 1.0 };
        let origin = color(0.0, 2.0, 0.0);
        let towards = |x: f32| (color(x, 0.0, 0.0) - origin).normalize();

        // 1.5 - ancho * 0.16 cae dentro de la banda; 1.0 y 1.7 caen fuera.
        assert!(ring_light(&active, origin, towards(1.5 - RING_WIDTH * 0.16), f32::INFINITY, 0.0).is_some());
        assert!(ring_light(&active, origin, towards(1.0), f32::INFINITY, 0.0).is_none());
        assert!(ring_light(&active, origin, towards(1.7), f32::INFINITY, 0.0).is_none());
    }

    #[test]
    fn the_sun_hides_rings_behind_it() {
        let ring = flat_ring();
        let active = ActiveRing { ring: &ring, radius: 1.5, width: RING_WIDTH, fade: 1.0 };
        let origin = color(0.0, 3.0, -1.5);
        let direction = (color(0.0, 0.0, 1.5 - RING_WIDTH * 0.16) - origin).normalize();

        assert!(ring_light(&active, origin, direction, f32::INFINITY, 0.0).is_some());
        assert!(ring_light(&active, origin, direction, 1.0, 0.0).is_none());
    }

    #[test]
    fn normals_are_spread_in_all_directions() {
        let rings = Rings::new();
        assert_eq!(rings.rings.len(), RING_COUNT + THIN_RING_COUNT);
        for axis in [color(1.0, 0.0, 0.0), color(0.0, 1.0, 0.0), color(0.0, 0.0, 1.0)] {
            assert!(rings.rings.iter().any(|ring| ring.normal.dot(axis).abs() > 0.8));
        }
    }

    #[test]
    fn thin_rings_are_thinner_and_reach_further() {
        let rings = Rings::new();
        let (thick, thin) = rings.rings.split_at(RING_COUNT);
        assert_eq!(thin.len(), THIN_RING_COUNT);
        assert!(thin[0].width < thick[0].width);
        assert!(thin[0].end_radius > thick[0].end_radius);
    }

    #[test]
    fn rings_fade_out_before_restarting() {
        let ring = flat_ring();
        let late = activate(&ring, 0.99 / EXPANSION_SPEED).map_or(0.0, |active| active.fade);
        assert!(late < 0.01);
    }
}
