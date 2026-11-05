//! Sol rojo. Su superficie es plasma que hierve: no recibe luz de afuera, su
//! color sale de un valor de calor calculado con ruido animado.

use noise::OpenSimplex;
use raylib::prelude::*;

use super::noise_helpers::fbm;
use super::{color, mix, smoothstep};
use crate::Uniforms;
use crate::fragment::Fragment;

/// Velocidad con la que el plasma cambia de forma.
const FLOW_SPEED: f32 = 0.14;
/// Tamaño de las masas de plasma: más alto da masas más pequeñas.
const PLASMA_SCALE: f32 = 3.6;
/// Cuánto se retuerce el plasma: 0 lo deja como manchas sin distorsión.
const TURBULENCE: f32 = 2.6;
/// Tamaño de las manchas frías.
const SPOT_SCALE: f32 = 2.4;
/// Valor del ruido desde el que hay mancha: más alto da menos manchas.
const SPOT_THRESHOLD: f32 = 0.60;
/// Cuánto se oscurecen las manchas: 0 no se notan, 1 quedan negras.
const SPOT_DARKNESS: f32 = 0.8;
/// Brillo del contorno del disco: 0 no brilla.
const RIM_GLOW: f32 = 0.85;
/// Ancho del contorno brillante: más alto lo hace más delgado.
const RIM_SHARPNESS: f32 = 2.6;

/// Calor de un punto de la superficie, de 0 (frío) a cerca de 1 (muy caliente).
fn heat(noise: &OpenSimplex, p: Vector3, time: f32) -> f32 {
    let t = time * FLOW_SPEED;

    // Distorsión: cada eje se desplaza según otro ruido que también se mueve.
    // Así el plasma se deforma en vez de deslizarse como un dibujo rígido.
    let warp = color(
        fbm(noise, p * 2.1 + color(t, 5.0, -t * 0.6), 3) - 0.5,
        fbm(noise, p * 2.1 + color(-8.0, t * 0.8, 3.0), 3) - 0.5,
        fbm(noise, p * 2.1 + color(t * 0.5, -t, 12.0), 3) - 0.5,
    ) * TURBULENCE;

    let plasma = fbm(noise, p * PLASMA_SCALE + warp + color(-t * 0.5, t, 0.0), 4);

    let raw = 0.18 + smoothstep(0.30, 0.70, plasma) * 0.62;

    // Manchas: zonas más frías que se mueven mucho más lento que el plasma.
    let spot_value = fbm(noise, p * SPOT_SCALE + warp * 0.25 + color(t * 0.2, 20.0, 0.0), 3);
    let spot = smoothstep(SPOT_THRESHOLD, SPOT_THRESHOLD + 0.10, spot_value);

    raw * (1.0 - spot * SPOT_DARKNESS)
}

/// Rampa de color: rojo casi negro, rojo puro y rojo encendido.
fn heat_color(value: f32) -> Vector3 {
    let cold = color(0.13, 0.0, 0.012);
    let red = color(0.95, 0.005, 0.005);
    let orange = color(1.0, 0.09, 0.015);
    let hot = color(1.0, 0.46, 0.30);

    let ramp = mix(cold, red, smoothstep(0.12, 0.55, value));
    let ramp = mix(ramp, orange, smoothstep(0.55, 0.70, value));
    mix(ramp, hot, smoothstep(0.68, 0.80, value))
}

pub fn shade(fragment: &Fragment, uniforms: &Uniforms) -> Vector3 {
    let p = fragment.object_position.normalize();
    let surface = heat_color(heat(&uniforms.noise, p, uniforms.time));

    // El contorno brilla. Depende de la cámara, no de una luz externa:
    // `intensity` no se usa aquí.
    let view = (uniforms.camera_position - fragment.world_position).normalize();
    let facing = fragment.normal.dot(view).max(0.0);
    let rim = (1.0 - facing).powf(RIM_SHARPNESS) * RIM_GLOW;
    mix(surface, color(1.0, 0.13, 0.04), rim)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(i: usize) -> Vector3 {
        let angle = i as f32 * 2.4;
        let y = 1.0 - i as f32 / 500.0;
        let r = (1.0 - y * y).max(0.0).sqrt();
        color(r * angle.cos(), y, r * angle.sin())
    }

    #[test]
    fn surface_changes_with_time() {
        let noise = OpenSimplex::new(7);
        let changed = (0..1000)
            .filter(|&i| (heat(&noise, point(i), 0.0) - heat(&noise, point(i), 6.0)).abs() > 0.05)
            .count();
        assert!(changed > 300, "solo cambiaron {changed} puntos");
    }

    #[test]
    fn surface_has_cold_and_hot_regions() {
        let noise = OpenSimplex::new(7);
        let values: Vec<f32> = (0..1000).map(|i| heat(&noise, point(i), 0.0)).collect();
        let coldest = values.iter().cloned().fold(f32::MAX, f32::min);
        let hottest = values.iter().cloned().fold(0.0, f32::max);
        assert!(hottest - coldest > 0.4, "de {coldest} a {hottest}");
    }

    #[test]
    fn color_ramp_gets_brighter_with_heat() {
        let brightness = |value: f32| {
            let ramp = heat_color(value);
            ramp.x + ramp.y + ramp.z
        };
        assert!(brightness(0.1) < brightness(0.5));
        assert!(brightness(0.5) < brightness(0.95));
    }
}
