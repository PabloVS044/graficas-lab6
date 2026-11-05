//! Planeta rocoso cubierto de selva: mar, costa, selva, ríos y mesetas de roca,
//! con una capa de nubes que se mueve por su cuenta y proyecta sombras. El agua
//! refleja la luz; la selva y la roca son mates. En el lado de noche la selva se
//! llena de luciérnagas y se ven las fogatas de los asentamientos.

use noise::OpenSimplex;
use raylib::prelude::*;

use super::noise_helpers::{cell_random, fbm, sample};
use super::{color, mix, smoothstep};
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
/// Altura de las nubes sobre el suelo (el radio del planeta es 1). Más alto
/// separa más la sombra de su nube.
const CLOUD_ALTITUDE: f32 = 0.07;
/// Tamaño del reflejo en el agua: más alto da un brillo más pequeño y concentrado.
const WATER_SHININESS: f32 = 70.0;
/// Fuerza del reflejo en el agua: 0 la deja tan mate como la selva.
const WATER_REFLECTION: f32 = 0.85;
/// Parte del reflejo del agua que conservan los ríos.
const RIVER_REFLECTION: f32 = 0.3;
/// Tamaño de las luciérnagas: más alto da puntos más pequeños y numerosos.
const FIREFLY_GRID: f32 = 72.0;
/// Fracción de celdas de la cuadrícula que tienen una luciérnaga.
const FIREFLY_CHANCE: f32 = 0.55;
/// Velocidad con la que parpadean.
const FIREFLY_BLINK_SPEED: f32 = 2.2;
/// Fuerza de su luz: 0 las apaga.
const FIREFLY_BRIGHTNESS: f32 = 1.5;
/// Tamaño de las zonas con asentamientos: más alto da más zonas y más pequeñas.
const SETTLEMENT_SCALE: f32 = 8.0;
/// Valor del ruido desde el que hay asentamiento: más alto da menos.
const SETTLEMENT_THRESHOLD: f32 = 0.56;
/// Tamaño de las fogatas: más alto da puntos más pequeños y numerosos.
const CAMPFIRE_GRID: f32 = 58.0;
/// Fracción de celdas de un asentamiento que tienen una fogata.
const CAMPFIRE_CHANCE: f32 = 0.75;
/// Velocidad del titileo del fuego.
const CAMPFIRE_FLICKER_SPEED: f32 = 9.0;
/// Fuerza de la luz del fuego: 0 las apaga.
const CAMPFIRE_BRIGHTNESS: f32 = 1.7;
/// Cuánto ocultan las nubes las luces de la noche.
const CLOUD_LIGHT_BLOCK: f32 = 0.8;
/// Cuánto oscurece la sombra: 0 no se ve, 1 deja el suelo negro.
const SHADOW_STRENGTH: f32 = 0.72;

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

/// Punto de la capa de nubes que queda entre `p` y la luz: se sale de `p` hacia
/// la luz hasta llegar a la esfera de radio `1 + CLOUD_ALTITUDE`.
///
/// `p` tiene largo 1, así que `|p + light * t|² = radio²` se reduce a
/// `t² + 2 * b * t + c = 0`, con `b = p · light` y `c = 1 - radio²`.
fn point_towards_light(p: Vector3, light: Vector3) -> Vector3 {
    let radius = 1.0 + CLOUD_ALTITUDE;
    let b = p.dot(light);
    let c = 1.0 - radius * radius;
    let t = -b + (b * b - c).sqrt();
    (p + light * t).normalize()
}

/// Reflejo especular (Blinn-Phong): es máximo cuando la normal coincide con el
/// punto medio entre la dirección de la luz y la de la cámara. Por eso, a
/// diferencia de `intensity`, cambia al mover la cámara.
fn specular(normal: Vector3, light: Vector3, view: Vector3, shininess: f32) -> f32 {
    // Sin esto el brillo aparecería en el lado que no recibe luz.
    if normal.dot(light) <= 0.0 {
        return 0.0;
    }
    let halfway = (light + view).normalize();
    normal.dot(halfway).max(0.0).powf(shininess)
}

/// 0 en el lado iluminado y 1 en el de noche, con una transición en el borde.
fn night_factor(normal: Vector3, light: Vector3) -> f32 {
    1.0 - smoothstep(-0.14, 0.05, normal.dot(light))
}

/// Luces diminutas repartidas por una cuadrícula. Algunas celdas tienen un punto
/// que parpadea. `steady` es el brillo que nunca pierde: en 0 se apaga del todo y
/// se mueve un poco; en 1 queda fijo y siempre encendido. Devuelve de 0 a 1.
#[allow(clippy::too_many_arguments)]
fn sparkle(
    p: Vector3,
    grid: f32,
    chance: f32,
    size: f32,
    salt: u32,
    time: f32,
    speed: f32,
    steady: f32,
) -> f32 {
    let cell = p * grid;
    let (cx, cy, cz) = (cell.x.floor(), cell.y.floor(), cell.z.floor());
    let seed = (cx as i32, cy as i32, cz as i32);

    if cell_random(seed, salt) > chance {
        return 0.0;
    }

    let phase = cell_random(seed, salt + 1) * std::f32::consts::TAU;
    let wander = (time * 0.7 + phase).sin() * 0.12 * (1.0 - steady);
    let dx = cell.x - cx - (0.25 + cell_random(seed, salt + 2) * 0.5) - wander;
    let dy = cell.y - cy - (0.25 + cell_random(seed, salt + 3) * 0.5) + wander;
    let dz = cell.z - cz - (0.25 + cell_random(seed, salt + 4) * 0.5);
    let distance = (dx * dx + dy * dy + dz * dz).sqrt();

    let glow = 1.0 - smoothstep(size * 0.3, size, distance);
    let blink = (time * speed + phase).sin() * 0.5 + 0.5;
    glow * (steady + (1.0 - steady) * blink)
}

/// Luz de las luciérnagas en un punto de selva. Vuelan en enjambres: un ruido de
/// baja frecuencia decide dónde hay muchas y dónde ninguna.
fn fireflies(noise: &OpenSimplex, p: Vector3, time: f32) -> Vector3 {
    let swarm = smoothstep(0.46, 0.60, fbm(noise, p * 6.5 + color(-9.0, 14.0, 3.0), 2));
    if swarm <= 0.0 {
        return Vector3::zero();
    }

    let light = sparkle(p, FIREFLY_GRID, FIREFLY_CHANCE, 0.42, 10, time, FIREFLY_BLINK_SPEED, 0.0);
    // Además de los puntos, el enjambre tiñe apenas la selva debajo.
    let haze = color(0.012, 0.035, 0.008);
    (color(0.72, 1.0, 0.22) * light * FIREFLY_BRIGHTNESS + haze) * swarm
}

/// Luz de los asentamientos: zonas repartidas con ruido donde hay fogatas muy
/// juntas, más un resplandor naranja que tiñe el suelo alrededor.
fn settlements(noise: &OpenSimplex, p: Vector3, time: f32) -> Vector3 {
    let value = fbm(noise, p * SETTLEMENT_SCALE + color(21.0, -6.0, 13.0), 3);
    let area = smoothstep(SETTLEMENT_THRESHOLD, SETTLEMENT_THRESHOLD + 0.06, value);
    if area <= 0.0 {
        return Vector3::zero();
    }

    let fire = sparkle(p, CAMPFIRE_GRID, CAMPFIRE_CHANCE, 0.36, 20, time, CAMPFIRE_FLICKER_SPEED, 0.6);
    let embers = color(0.10, 0.034, 0.006);
    (color(1.0, 0.52, 0.10) * fire * CAMPFIRE_BRIGHTNESS + embers) * area
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

    // Máscara de agua: el mar y, con menos brillo, los ríos de las tierras bajas.
    // A pleno brillo un río tan delgado se ve como un alambre blanco.
    let water = (1.0 - land) + land * river * lowland * (1.0 - highland) * RIVER_REFLECTION;

    let surface = mix(sea, ground, land);

    // Sombra: si hay una nube en el camino hacia la luz, el suelo recibe menos.
    // La esfera no se transforma, así que objeto y mundo coinciden y la luz se
    // puede usar tal cual. Se recalcula el ruido en ese otro punto, por lo que
    // la sombra se mueve con la nube y cambia con la dirección de la luz.
    let above = point_towards_light(p, uniforms.light_direction);
    let shadow = cloud_density(noise, above, uniforms.time, CLOUD_COVERAGE);
    let sunlight = 1.0 - shadow * SHADOW_STRENGTH;
    let surface = surface * sunlight;

    // Las nubes reciben la misma luz que la superficie que tapan.
    let cloud = cloud_density(noise, p, uniforms.time, CLOUD_COVERAGE) * CLOUD_OPACITY;
    let lit = mix(surface, color(0.66, 0.70, 0.68), cloud);

    // El reflejo solo aparece en el agua. Las nubes lo tapan y su sombra lo apaga.
    let view = (uniforms.camera_position - fragment.world_position).normalize();
    let glint = specular(
        fragment.normal,
        uniforms.light_direction,
        view,
        WATER_SHININESS,
    ) * WATER_REFLECTION
        * water
        * sunlight
        * (1.0 - cloud);

    // Luces de la noche: no dependen del sol, así que se suman después de
    // iluminar, y solo existen del lado oscuro. Se calculan solo si hacen falta.
    let night = night_factor(fragment.normal, uniforms.light_direction);
    let jungle_only = land * (1.0 - highland) * (1.0 - river * lowland);
    let night_lights = if night > 0.0 && jungle_only > 0.0 {
        let lights = fireflies(noise, p, uniforms.time) + settlements(noise, p, uniforms.time);
        lights * night * jungle_only * (1.0 - cloud * CLOUD_LIGHT_BLOCK)
    } else {
        Vector3::zero()
    };

    lit * (0.1 + fragment.intensity * 0.9) + color(1.0, 0.95, 0.82) * glint + night_lights
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
    fn shadow_point_is_straight_above_when_light_is_overhead() {
        let p = color(0.0, 1.0, 0.0);
        assert!((point_towards_light(p, p) - p).length() < 1e-5);
    }

    #[test]
    fn shadow_shifts_more_when_light_is_low() {
        let p = color(0.0, 1.0, 0.0);
        let high = point_towards_light(p, color(0.2, 0.98, 0.0).normalize());
        let low = point_towards_light(p, color(0.95, 0.3, 0.0).normalize());

        // La sombra de una nube cae al lado contrario de la luz: la nube que
        // tapa a `p` está corrida hacia la luz.
        assert!(high.x > 0.0 && low.x > high.x);
    }

    #[test]
    fn specular_peaks_at_mirror_angle_and_ignores_the_dark_side() {
        let normal = color(0.0, 1.0, 0.0);
        let light = color(0.5, 0.8, 0.0).normalize();
        let mirror = color(-0.5, 0.8, 0.0).normalize();
        let away = color(0.9, 0.3, 0.0).normalize();

        assert!(specular(normal, light, mirror, 70.0) > 0.99);
        assert!(specular(normal, light, away, 70.0) < 0.2);
        assert_eq!(specular(normal, -light, mirror, 70.0), 0.0);
    }

    #[test]
    fn night_lights_only_exist_on_the_dark_side() {
        let light = color(0.0, 1.0, 0.0);
        assert_eq!(night_factor(light, light), 0.0);
        assert_eq!(night_factor(-light, light), 1.0);
    }

    #[test]
    fn sparkles_blink_over_time() {
        let p = color(0.6, 0.2, 0.77).normalize();
        let values: Vec<f32> = (0..2000)
            .map(|i| {
                let p = (p + color(i as f32 * 0.002, 0.0, 0.0)).normalize();
                sparkle(p, 85.0, 1.0, 0.34, 10, 0.0, 2.2, 0.0) - sparkle(p, 85.0, 1.0, 0.34, 10, 1.5, 2.2, 0.0)
            })
            .collect();
        assert!(values.iter().any(|difference| difference.abs() > 0.2));
    }

    #[test]
    fn steady_lights_never_go_out() {
        // Centro aproximado de una luz: se busca el punto más brillante.
        let brightest = |time: f32, steady: f32| {
            (0..4000)
                .map(|i| {
                    let angle = i as f32 * 0.01;
                    let p = color(angle.cos(), 0.1, angle.sin()).normalize();
                    sparkle(p, 58.0, 1.0, 0.36, 20, time, 9.0, steady)
                })
                .fold(0.0, f32::max)
        };
        assert!(brightest(0.3, 1.0) > 0.9);
        assert!(brightest(4.1, 1.0) > 0.9);
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
