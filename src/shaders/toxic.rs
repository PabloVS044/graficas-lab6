//! Mundo tóxico: gigante gaseoso de atmósfera envenenada, con bandas de verde
//! ácido, ceniza y lodo que fluyen como un líquido, y una tormenta que las curva.
//! Entre el gas flotan los restos de lo que fue una ciudad: piezas de cemento gris.

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
const STORM_CENTER: Vector3 = Vector3 { x: -0.36, y: 0.16, z: 0.92 };
/// Radio de la zona que se curva, en radianes sobre la esfera.
const STORM_RADIUS: f32 = 0.62;
/// Vueltas que dan las bandas en el centro, en radianes: 0 las deja rectas.
const STORM_TWIST: f32 = 5.5;
/// Tamaño de los restos de tierra: más alto da pedazos más pequeños.
const RUIN_SCALE: f32 = 2.5;
/// Valor del ruido desde el que hay tierra: más alto deja menos restos.
const RUIN_THRESHOLD: f32 = 0.55;
/// Cuánto se rompe el borde de los restos.
const RUIN_BREAKUP: f32 = 0.22;
/// Fuerza del brillo tóxico en la orilla de los restos: 0 lo apaga.
const SHORE_GLOW: f32 = 0.35;

/// Gira `p` alrededor de la tormenta. El giro es máximo en el centro y llega a 0
/// en el borde, por eso las bandas de afuera siguen rectas y se conectan sin
/// cortes con las curvas de adentro.
fn swirl(p: Vector3, center: Vector3) -> Vector3 {
    let distance = p.dot(center).clamp(-1.0, 1.0).acos();
    if distance >= STORM_RADIUS {
        return p;
    }

    let falloff = 1.0 - distance / STORM_RADIUS;
    let (sin, cos) = (STORM_TWIST * falloff * falloff).sin_cos();

    // Fórmula de Rodrigues: rotación de `p` alrededor del eje `center`.
    p * cos + center.cross(p) * sin + center * (center.dot(p) * (1.0 - cos))
}

/// Dónde hay restos de tierra. Un ruido grande decide los pedazos y uno fino rompe
/// sus bordes. Devuelve la cobertura, de 0 a 1, y qué tan lejos está de la orilla.
fn ruin_mask(noise: &OpenSimplex, p: Vector3) -> (f32, f32) {
    let large = fbm(noise, p * RUIN_SCALE + color(17.0, -3.0, 8.0), 4);
    let broken = fbm(noise, p * 15.0 + color(2.0, 30.0, -11.0), 2) - 0.5;
    let value = large + broken * RUIN_BREAKUP;
    (
        smoothstep(RUIN_THRESHOLD, RUIN_THRESHOLD + 0.012, value),
        value - RUIN_THRESHOLD,
    )
}

/// Color de los restos: cemento gris de una sola pieza, gastado por manchas
/// grandes y grano fino.
fn ruin_color(noise: &OpenSimplex, p: Vector3) -> Vector3 {
    let wear = fbm(noise, p * 9.0 + color(5.0, 12.0, -7.0), 3);
    let grain = fbm(noise, p * 46.0, 2);
    let tone = 0.20 + smoothstep(0.30, 0.70, wear) * 0.26 + (grain - 0.5) * 0.14;
    color(tone, tone * 0.98, tone * 0.93)
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
    let time = uniforms.time;
    let p = fragment.object_position.normalize();

    // Se calculan las bandas en el punto girado: la tormenta no se dibuja encima,
    // sino que curva el patrón que ya existe.
    let swirled = swirl(p, STORM_CENTER.normalize());
    let value = band_value(noise, swirled, time);
    let atmosphere = band_color(value);

    // Vetas de ceniza: ruido muy estirado que apaga parte de las bandas.
    let streak_point = color(swirled.x * 1.6, swirled.y * 8.0, swirled.z * 1.6);
    let streaks = smoothstep(0.50, 0.68, fbm(noise, streak_point + color(time * 0.03, 40.0, 0.0), 3));
    let mut surface = atmosphere * (1.0 - streaks * ASH_AMOUNT);
    let mut emission = Vector3::zero();

    // Los restos no se mueven y el gas sí: las bandas más densas les pasan por
    // encima y los van tapando y destapando.
    let (ruin, inland) = ruin_mask(noise, p);
    if ruin > 0.0 {
        let cover = smoothstep(0.62, 0.95, value) * 0.85;
        surface = mix(surface, ruin_color(noise, p), ruin * (1.0 - cover));

        // La orilla brilla como si el veneno estuviera corroyendo la tierra.
        let shore = ruin * (1.0 - smoothstep(0.0, 0.02, inland));
        emission += color(0.42, 0.95, 0.12) * shore * SHORE_GLOW * (1.0 - cover);
    }

    surface * (0.1 + fragment.intensity * 0.9) + emission
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
    fn ruins_cover_only_part_of_the_planet() {
        let noise = OpenSimplex::new(7);
        let covered = (0..2000).filter(|&i| ruin_mask(&noise, point(i)).0 > 0.5).count();
        assert!((100..1000).contains(&covered), "cubren {covered} de 2000 puntos");
    }

    #[test]
    fn ruins_stay_in_place_while_the_gas_moves() {
        // `ruin_mask` no recibe el tiempo: solo puede cambiar el gas.
        let noise = OpenSimplex::new(7);
        let p = point(700);
        assert_eq!(ruin_mask(&noise, p), ruin_mask(&noise, p));
        assert_ne!(band_value(&noise, p, 0.0), band_value(&noise, p, 30.0));
    }

    #[test]
    fn swirl_leaves_points_outside_the_storm_untouched() {
        let center = STORM_CENTER.normalize();
        let far = center * -1.0;
        assert!((swirl(far, center) - far).length() < 1e-6);
    }

    #[test]
    fn swirl_rotates_inside_keeping_the_distance_to_the_center() {
        let center = STORM_CENTER.normalize();
        let near = (center + color(0.15, 0.05, 0.0)).normalize();
        let swirled = swirl(near, center);

        assert!((swirled - near).length() > 0.05);
        assert!((swirled.dot(center) - near.dot(center)).abs() < 1e-4);
    }

    /// El giro debe llegar a 0 en el borde: si no, las bandas tendrían un corte.
    #[test]
    fn swirl_is_continuous_at_the_storm_edge() {
        let center = STORM_CENTER.normalize();
        let side = center.cross(color(0.0, 1.0, 0.0)).normalize();
        let at = |angle: f32| center * angle.cos() + side * angle.sin();
        let just_inside = at(STORM_RADIUS - 0.001);
        assert!((swirl(just_inside, center) - just_inside).length() < 1e-3);
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
