//! Planeta rocoso cubierto de selva: mar, costa, selva, ríos y mesetas de roca,
//! con una capa de nubes que se mueve por su cuenta.

use noise::OpenSimplex;
use raylib::prelude::*;

use super::noise_helpers::{fbm, sample};
use super::{mix, smoothstep};
use crate::Uniforms;
use crate::fragment::Fragment;

/// Tamaño de los continentes: más alto da más masas de tierra y más pequeñas.
const CONTINENT_SCALE: f32 = 2.3;
/// Altura del mar: más alto deja menos tierra.
const SEA_LEVEL: f32 = 0.495;
/// Altura donde la selva termina y empieza la roca.
const HIGHLAND_LEVEL: f32 = 0.62;
/// Tamaño de las manchas de copas de árboles.
const CANOPY_SCALE: f32 = 26.0;
/// Densidad de los ríos.
const RIVER_SCALE: f32 = 5.0;
/// Grosor de los ríos.
const RIVER_WIDTH: f32 = 0.012;
/// Parte del cielo cubierta por nubes: 0 despejado, 1 casi todo cubierto.
const CLOUD_COVERAGE: f32 = 0.42;
/// Tamaño de las nubes: más alto da nubes más pequeñas.
const CLOUD_SCALE: f32 = 3.4;
/// Radianes por segundo que giran las nubes. El terreno no se mueve.
const CLOUD_SPEED: f32 = 0.06;
/// Borde de las nubes: más alto da bordes más difusos.
const CLOUD_SOFTNESS: f32 = 0.07;
/// Opacidad de la parte más densa de una nube.
const CLOUD_OPACITY: f32 = 0.92;

fn color(r: f32, g: f32, b: f32) -> Vector3 {
    Vector3::new(r, g, b)
}

/// Densidad de nubes en un punto de la esfera, de 0 (cielo) a 1 (nube).
///
/// Las nubes giran alrededor del eje `y` en vez de desplazarse, para que no se
/// deformen en los polos. Una segunda capa de ruido distorsiona a la primera y
/// les da forma de jirones.
fn cloud_density(noise: &OpenSimplex, p: Vector3, time: f32, coverage: f32) -> f32 {
    let (sin, cos) = (time * CLOUD_SPEED).sin_cos();
    let moved = color(p.x * cos + p.z * sin, p.y, p.z * cos - p.x * sin);

    let warp = fbm(noise, moved * 2.6 + color(31.0, 17.0, 5.0), 3) - 0.5;
    let shape = fbm(noise, moved * CLOUD_SCALE + color(warp, -warp, warp) * 2.2, 5);

    // Más cobertura baja el umbral: más valores del ruido cuentan como nube.
    let threshold = 0.70 - coverage * 0.40;
    smoothstep(threshold - CLOUD_SOFTNESS, threshold + CLOUD_SOFTNESS, shape)
}

pub fn shade(fragment: &Fragment, uniforms: &Uniforms) -> Vector3 {
    let noise = &uniforms.noise;
    let p = fragment.object_position.normalize();

    // Una sola altura decide mar, tierra y roca.
    let height = fbm(noise, p * CONTINENT_SCALE, 5);
    let land = smoothstep(SEA_LEVEL - 0.004, SEA_LEVEL + 0.004, height);

    // Mar: más claro cuanto más cerca de la costa.
    let shallow = smoothstep(SEA_LEVEL - 0.09, SEA_LEVEL, height);
    let sea = mix(color(0.01, 0.06, 0.09), color(0.04, 0.30, 0.30), shallow);

    // Selva: la humedad elige el tono y las copas agregan el grano fino.
    let humidity = fbm(noise, p * 3.7 + color(11.0, 4.0, 7.0), 3);
    let canopy = fbm(noise, p * CANOPY_SCALE, 3);
    let jungle = mix(
        color(0.015, 0.13, 0.05),
        color(0.20, 0.42, 0.07),
        smoothstep(0.40, 0.62, humidity),
    );
    let jungle = jungle * (0.7 + smoothstep(0.32, 0.68, canopy) * 0.9);

    // Ríos: donde el ruido cruza por 0 se forma una línea continua. La
    // distorsión evita que se vean como curvas limpias.
    let warp = sample(noise, p * 14.0) * 0.12;
    let river_line = sample(noise, p * RIVER_SCALE + color(warp, 23.0, -warp)).abs();
    let river = 1.0 - smoothstep(RIVER_WIDTH * 0.5, RIVER_WIDTH, river_line);
    let lowland = 1.0 - smoothstep(SEA_LEVEL + 0.05, HIGHLAND_LEVEL - 0.03, height);
    let jungle = mix(jungle, color(0.04, 0.17, 0.15), river * lowland);

    // Mesetas: franjas horizontales según la altura, como estratos de roca.
    let strata = (height * 170.0 + canopy * 4.0).sin() * 0.5 + 0.5;
    let rock = mix(color(0.36, 0.22, 0.11), color(0.62, 0.45, 0.25), strata);
    let highland = smoothstep(HIGHLAND_LEVEL - 0.015, HIGHLAND_LEVEL + 0.015, height);
    let ground = mix(jungle, rock, highland);

    let surface = mix(sea, ground, land);

    // Las nubes reciben la misma luz que la superficie que tapan.
    let cloud = cloud_density(noise, p, uniforms.time, CLOUD_COVERAGE) * CLOUD_OPACITY;
    let lit = mix(surface, color(0.66, 0.70, 0.68), cloud);

    lit * (0.1 + fragment.intensity * 0.9)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Promedia la densidad sobre varios puntos de la esfera.
    fn average_density(time: f32, coverage: f32) -> f32 {
        let noise = OpenSimplex::new(7);
        let mut total = 0.0;
        for i in 0..2000 {
            let angle = i as f32 * 2.4;
            let y = 1.0 - i as f32 / 1000.0;
            let r = (1.0 - y * y).max(0.0).sqrt();
            let p = color(r * angle.cos(), y, r * angle.sin());
            total += cloud_density(&noise, p, time, coverage);
        }
        total / 2000.0
    }

    #[test]
    fn more_coverage_means_more_clouds() {
        assert!(average_density(0.0, 0.2) < average_density(0.0, 0.8));
    }

    #[test]
    fn clouds_move_with_time() {
        let noise = OpenSimplex::new(7);
        let p = color(0.6, 0.2, 0.77);
        let moved = (0..50).any(|i| {
            let time = i as f32;
            (cloud_density(&noise, p, time, 0.5) - cloud_density(&noise, p, 0.0, 0.5)).abs() > 0.2
        });
        assert!(moved);
    }
}
