//! Mundo tóxico: gigante gaseoso de atmósfera envenenada, con bandas de verde
//! ácido, ceniza y lodo que fluyen como un líquido.

use noise::OpenSimplex;
use raylib::prelude::*;

use super::noise_helpers::fbm;
use super::{color, mix, smoothstep};
use crate::Uniforms;
use crate::fragment::Fragment;

/// Cantidad de bandas: más alto da bandas más angostas.
const BAND_FREQUENCY: f32 = 11.0;
/// Cuánto ondulan los bordes: 0 deja líneas rectas, más alto las vuelve remolinos.
const TURBULENCE: f32 = 11.0;
/// Velocidad con la que fluye la atmósfera.
const FLOW_SPEED: f32 = 0.05;
/// Cuánto se estira el ruido a lo ancho: más alto da vetas más horizontales.
const STRETCH: f32 = 1.8;
/// Cuánto oscurecen las vetas de ceniza: 0 no se ven.
const ASH_AMOUNT: f32 = 0.35;

/// Desplazamiento de las bandas en un punto. Una capa de ruido distorsiona a la
/// otra, y eso hace que los bordes parezcan líquido en vez de una curva suave.
fn flow(noise: &OpenSimplex, p: Vector3, time: f32) -> f32 {
    let t = time * FLOW_SPEED;
    let stretched = color(p.x, p.y * STRETCH, p.z);

    let first = fbm(noise, stretched * 1.1 + color(t, 3.0, -t * 0.4), 2) - 0.5;
    let moved = stretched * 1.7 + color(first * 3.5 - t * 0.7, first * 2.0, t * 0.3);
    fbm(noise, moved, 2) - 0.5
}

/// Valor de la atmósfera, de 0 (lodo oscuro) a 1 (ácido encendido).
fn band_value(noise: &OpenSimplex, p: Vector3, time: f32) -> f32 {
    let offset = flow(noise, p, time) * TURBULENCE;
    let wide = (p.y * BAND_FREQUENCY + offset).sin() * 0.5 + 0.5;
    let fine = (p.y * BAND_FREQUENCY * 2.7 + offset * 1.6 + 1.1).sin() * 0.5 + 0.5;
    wide * 0.78 + fine * 0.22
}

/// Rampa de color: lodo, ceniza, verde ácido y amarillo enfermizo.
fn band_color(value: f32) -> Vector3 {
    let sludge = color(0.035, 0.065, 0.03);
    let ash = color(0.25, 0.30, 0.19);
    let moss = color(0.20, 0.46, 0.07);
    let acid = color(0.46, 0.86, 0.10);
    let sick = color(0.90, 0.95, 0.32);

    let ramp = mix(sludge, ash, smoothstep(0.05, 0.34, value));
    let ramp = mix(ramp, moss, smoothstep(0.32, 0.52, value));
    let ramp = mix(ramp, acid, smoothstep(0.50, 0.72, value));
    mix(ramp, sick, smoothstep(0.74, 0.97, value))
}

pub fn shade(fragment: &Fragment, uniforms: &Uniforms) -> Vector3 {
    let noise = &uniforms.noise;
    let p = fragment.object_position.normalize();

    let atmosphere = band_color(band_value(noise, p, uniforms.time));

    // Vetas de ceniza: ruido muy estirado que apaga parte de las bandas.
    let streak_point = color(p.x * 1.6, p.y * 8.0, p.z * 1.6);
    let streaks = smoothstep(0.50, 0.68, fbm(noise, streak_point + color(uniforms.time * 0.03, 40.0, 0.0), 3));
    let surface = atmosphere * (1.0 - streaks * ASH_AMOUNT);

    surface * (0.1 + fragment.intensity * 0.9)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atmosphere_flows_with_time() {
        let noise = OpenSimplex::new(7);
        let p = color(0.6, 0.2, 0.77).normalize();
        let moved = (1..40).any(|i| {
            (band_value(&noise, p, i as f32) - band_value(&noise, p, 0.0)).abs() > 0.1
        });
        assert!(moved);
    }

    /// Bandas: el valor cambia mucho más al subir que al rodear el planeta.
    #[test]
    fn value_changes_more_with_latitude_than_with_longitude() {
        let noise = OpenSimplex::new(7);
        let point = |latitude: f32, longitude: f32| {
            color(
                latitude.cos() * longitude.cos(),
                latitude.sin(),
                latitude.cos() * longitude.sin(),
            )
        };
        let (mut vertical, mut horizontal) = (0.0, 0.0);
        for i in 0..400 {
            let latitude = -1.0 + i as f32 * 0.005;
            let longitude = i as f32 * 0.37;
            let here = band_value(&noise, point(latitude, longitude), 0.0);
            vertical += (band_value(&noise, point(latitude + 0.02, longitude), 0.0) - here).abs();
            horizontal += (band_value(&noise, point(latitude, longitude + 0.02), 0.0) - here).abs();
        }
        assert!(vertical > horizontal * 1.5, "{vertical} contra {horizontal}");
    }
}
