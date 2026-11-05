//! Mundo tóxico: gigante gaseoso de atmósfera envenenada, con bandas de verde
//! ácido, ceniza y lodo que fluyen como un líquido, y una tormenta radiactiva
//! que curva las bandas a su alrededor.

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
/// Dirección del centro de la tormenta, desde el centro del planeta.
const VORTEX_CENTER: Vector3 = Vector3 { x: -0.36, y: 0.16, z: 0.92 };
/// Radio de la zona que se curva, en radianes sobre la esfera.
const VORTEX_RADIUS: f32 = 0.62;
/// Vueltas que dan las bandas en el centro, en radianes: 0 las deja rectas.
const VORTEX_TWIST: f32 = 5.5;
/// Radio del ojo amarillo, en radianes.
const EYE_RADIUS: f32 = 0.13;
/// Ancho de la pared oscura que rodea al ojo, en radianes.
const STORM_WALL_WIDTH: f32 = 0.20;
/// Velocidad de giro de los brazos del ojo.
const EYE_SPIN_SPEED: f32 = 1.1;
/// Luz propia del ojo: 0 hace que de noche no se vea.
const EYE_GLOW: f32 = 0.45;

/// Gira `p` alrededor de la tormenta. El giro es máximo en el centro y llega a 0
/// en el borde, por eso las bandas de afuera siguen rectas y se conectan sin
/// cortes con las curvas de adentro. Devuelve el punto girado y la distancia al
/// centro en radianes.
fn swirl(p: Vector3) -> (Vector3, f32) {
    let center = VORTEX_CENTER.normalize();
    let distance = p.dot(center).clamp(-1.0, 1.0).acos();
    if distance >= VORTEX_RADIUS {
        return (p, distance);
    }

    let falloff = 1.0 - distance / VORTEX_RADIUS;
    let (sin, cos) = (VORTEX_TWIST * falloff * falloff).sin_cos();

    // Fórmula de Rodrigues: rotación de `p` alrededor del eje `center`.
    let rotated = p * cos + center.cross(p) * sin + center * (center.dot(p) * (1.0 - cos));
    (rotated, distance)
}

/// Pinta el ojo y su pared oscura sobre la atmósfera. Dentro del ojo unos brazos
/// giran con el tiempo, sin deformarse, y por eso no se enredan.
fn storm_eye(atmosphere: Vector3, p: Vector3, distance: f32, time: f32) -> (Vector3, f32) {
    let center = VORTEX_CENTER.normalize();
    let wall_end = EYE_RADIUS + STORM_WALL_WIDTH;
    if distance >= wall_end {
        return (atmosphere, 0.0);
    }

    // Ángulo alrededor de la tormenta, medido en el plano tangente a su centro.
    let helper = if center.y.abs() < 0.9 { color(0.0, 1.0, 0.0) } else { color(1.0, 0.0, 0.0) };
    let tangent = center.cross(helper).normalize();
    let bitangent = center.cross(tangent);
    let angle = p.dot(bitangent).atan2(p.dot(tangent));

    let arms = (angle * 3.0 + distance * 34.0 - time * EYE_SPIN_SPEED).sin() * 0.5 + 0.5;

    let wall = (1.0 - smoothstep(EYE_RADIUS, wall_end, distance)) * 0.9;
    let eye = 1.0 - smoothstep(EYE_RADIUS * 0.75, EYE_RADIUS * 1.15, distance);

    let dark = color(0.02, 0.035, 0.015);
    let core = mix(color(0.95, 0.72, 0.08), color(1.0, 0.98, 0.42), arms);

    (mix(mix(atmosphere, dark, wall), core, eye), eye)
}

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

    // Se calculan las bandas en el punto girado: la tormenta no se dibuja encima,
    // sino que curva el patrón que ya existe.
    let (swirled, distance) = swirl(p);
    let atmosphere = band_color(band_value(noise, swirled, uniforms.time));

    // Vetas de ceniza: ruido muy estirado que apaga parte de las bandas.
    let streak_point = color(swirled.x * 1.6, swirled.y * 8.0, swirled.z * 1.6);
    let streaks = smoothstep(0.50, 0.68, fbm(noise, streak_point + color(uniforms.time * 0.03, 40.0, 0.0), 3));
    let surface = atmosphere * (1.0 - streaks * ASH_AMOUNT);

    let (surface, eye) = storm_eye(surface, p, distance, uniforms.time);

    // El ojo también tiene luz propia, así que se ve aunque esté de noche.
    surface * (0.1 + fragment.intensity * 0.9) + color(1.0, 0.86, 0.2) * eye * EYE_GLOW
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swirl_leaves_points_outside_the_storm_untouched() {
        let far = (VORTEX_CENTER.normalize() * -1.0).normalize();
        let (swirled, distance) = swirl(far);
        assert!((swirled - far).length() < 1e-6);
        assert!(distance >= VORTEX_RADIUS);
    }

    #[test]
    fn swirl_rotates_inside_keeping_the_distance_to_the_center() {
        let center = VORTEX_CENTER.normalize();
        let near = (center + color(0.15, 0.05, 0.0)).normalize();
        let (swirled, distance) = swirl(near);

        assert!(distance < VORTEX_RADIUS);
        assert!((swirled - near).length() > 0.05);
        assert!((swirled.dot(center) - near.dot(center)).abs() < 1e-4);
    }

    /// El giro debe llegar a 0 en el borde: si no, las bandas tendrían un corte.
    #[test]
    fn swirl_is_continuous_at_the_storm_edge() {
        let center = VORTEX_CENTER.normalize();
        let side = center.cross(color(0.0, 1.0, 0.0)).normalize();
        let at = |angle: f32| center * angle.cos() + side * angle.sin();
        let just_inside = at(VORTEX_RADIUS - 0.001);
        assert!((swirl(just_inside).0 - just_inside).length() < 1e-3);
    }

    #[test]
    fn eye_is_bright_and_only_near_the_center() {
        let center = VORTEX_CENTER.normalize();
        let base = color(0.2, 0.5, 0.1);
        let (at_center, eye) = storm_eye(base, center, 0.0, 0.0);
        assert!(eye > 0.99 && at_center.x > 0.9 && at_center.y > 0.7);

        let (outside, eye) = storm_eye(base, center, EYE_RADIUS + STORM_WALL_WIDTH + 0.01, 0.0);
        assert_eq!(eye, 0.0);
        assert!((outside - base).length() < 1e-6);
    }

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
