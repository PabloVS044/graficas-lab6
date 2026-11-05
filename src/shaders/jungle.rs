//! Planeta rocoso cubierto de selva. Por ahora solo decide las regiones de la
//! superficie: mar, costa, selva, ríos y mesetas de roca.

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

fn color(r: f32, g: f32, b: f32) -> Vector3 {
    Vector3::new(r, g, b)
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

    mix(sea, ground, land) * (0.1 + fragment.intensity * 0.9)
}
