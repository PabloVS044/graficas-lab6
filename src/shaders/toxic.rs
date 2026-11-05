//! Mundo tóxico: gigante gaseoso de atmósfera envenenada, con bandas de verde
//! ácido, ceniza y lodo que fluyen como un líquido. Dos ojos parpadean en medio
//! de sendas tormentas que curvan las bandas: uno bonito en el lado iluminado y
//! otro tenebroso en el lado oscuro, que sigue a la cámara.

use noise::OpenSimplex;
use raylib::prelude::*;

use super::noise_helpers::{fbm, sample};
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
/// Radio de la zona que se curva alrededor de cada ojo, en radianes sobre la esfera.
const VORTEX_RADIUS: f32 = 0.62;
/// Vueltas que dan las bandas junto al ojo, en radianes: 0 las deja rectas.
const VORTEX_TWIST: f32 = 5.5;
/// Centro del ojo bonito, en el lado iluminado.
const DAY_EYE_CENTER: Vector3 = Vector3 { x: 0.10, y: 0.12, z: 0.99 };
/// Centro del ojo tenebroso, en el lado oscuro.
const NIGHT_EYE_CENTER: Vector3 = Vector3 { x: -0.55, y: -0.25, z: -0.80 };
/// Mitad del ancho de un ojo, en radianes.
const EYE_WIDTH: f32 = 0.27;
/// Mitad de la altura de un ojo abierto, en radianes.
const EYE_HEIGHT: f32 = 0.135;
/// Radio del iris.
const IRIS_RADIUS: f32 = 0.105;
/// Segundos entre parpadeos del ojo bonito y lo que dura cada uno.
const DAY_BLINK_PERIOD: f32 = 4.5;
const DAY_BLINK_LENGTH: f32 = 0.35;
/// Parpadeos del ojo tenebroso: lentos, y dos ritmos distintos los vuelven irregulares.
const NIGHT_BLINK_PERIOD: f32 = 6.5;
const NIGHT_BLINK_LENGTH: f32 = 1.0;
const NIGHT_SECOND_BLINK_PERIOD: f32 = 11.0;
const NIGHT_SECOND_BLINK_LENGTH: f32 = 0.5;
/// Cuánto se mueve el iris del ojo tenebroso para seguir a la cámara.
const NIGHT_GAZE_REACH: f32 = 0.11;
/// Luz propia del ojo bonito: lo mantiene visible cerca del borde de la sombra.
const DAY_EYE_GLOW: f32 = 0.22;
/// Fuerza del resplandor rojo alrededor del ojo tenebroso.
const NIGHT_HALO: f32 = 0.14;

/// Gira `p` alrededor de un centro. El giro es máximo junto al centro y llega a 0
/// en el borde, por eso las bandas de afuera siguen rectas y se conectan sin
/// cortes con las curvas de adentro.
fn swirl(p: Vector3, center: Vector3) -> Vector3 {
    let distance = p.dot(center).clamp(-1.0, 1.0).acos();
    if distance >= VORTEX_RADIUS {
        return p;
    }

    let falloff = 1.0 - distance / VORTEX_RADIUS;
    let (sin, cos) = (VORTEX_TWIST * falloff * falloff).sin_cos();

    // Fórmula de Rodrigues: rotación de `p` alrededor del eje `center`.
    p * cos + center.cross(p) * sin + center * (center.dot(p) * (1.0 - cos))
}

/// Ejes de un plano tangente a la esfera en `center`: el primero es horizontal y
/// el segundo apunta hacia arriba, para que los ojos queden derechos.
fn eye_axes(center: Vector3) -> (Vector3, Vector3) {
    let horizontal = color(0.0, 1.0, 0.0).cross(center).normalize();
    (horizontal, center.cross(horizontal))
}

/// Apertura del párpado: 1 abierto y 0 cerrado. Cierra y abre una vez por período.
fn blink(time: f32, period: f32, length: f32) -> f32 {
    let t = (time / period).fract() * period;
    if t >= length {
        return 1.0;
    }
    smoothstep(0.0, 1.0, (t / length * 2.0 - 1.0).abs())
}

/// Forma de un ojo en coordenadas `u`, `v` del plano tangente.
struct EyeShape {
    /// Parte visible del globo: 1 adentro de los párpados abiertos.
    opening: f32,
    /// Almendra completa, con el ojo abierto o cerrado.
    almond: f32,
    /// Sombra alrededor de la almendra.
    socket: f32,
}

fn eye_shape(u: f32, v: f32, open: f32) -> EyeShape {
    let across = (u / EYE_WIDTH).abs();
    let height = EYE_HEIGHT * (1.0 - across * across).max(0.0);

    // Ancho del contorno suave, para que el borde no se vea dentado.
    let soft = 0.006;
    let edge = |limit: f32| 1.0 - smoothstep(limit - soft, limit + soft, v.abs());
    let inside = if across < 1.0 { 1.0 } else { 0.0 };

    EyeShape {
        opening: edge(height * open) * inside * smoothstep(0.0, 0.12, open),
        almond: edge(height) * inside,
        socket: (1.0 - smoothstep(0.0, 0.11, v.abs() - height)) * (1.0 - smoothstep(1.0, 1.35, across)),
    }
}

/// Ojo del lado iluminado: esclerótica limpia, iris esmeralda con fibras, pupila
/// redonda y un brillo. Mira un poco a los lados. Devuelve el color y su máscara.
fn day_eye(u: f32, v: f32, time: f32) -> (Vector3, f32) {
    let shape = eye_shape(u, v, blink(time, DAY_BLINK_PERIOD, DAY_BLINK_LENGTH));
    if shape.socket <= 0.0 {
        return (Vector3::zero(), 0.0);
    }

    let du = u - (time * 0.31).sin() * 0.035;
    let dv = v - (time * 0.23).sin() * 0.012;
    let radius = (du * du + dv * dv).sqrt() / IRIS_RADIUS;
    let angle = dv.atan2(du);

    let fibers = (angle * 22.0).sin() * 0.5 + (angle * 37.0 + 2.0).sin() * 0.5;
    let iris = mix(color(0.10, 0.86, 0.62), color(0.02, 0.26, 0.38), smoothstep(0.35, 1.0, radius));
    let iris = iris * (0.85 + fibers * 0.18);
    let iris = iris * (1.0 - smoothstep(0.82, 1.0, radius) * 0.75);

    let sclera = color(0.93, 0.97, 0.90) * (1.0 - (v.abs() / EYE_HEIGHT) * 0.28);
    let globe = mix(iris, sclera, smoothstep(0.97, 1.03, radius));
    let globe = mix(color(0.0, 0.02, 0.03), globe, smoothstep(0.34, 0.42, radius));

    // Brillo: un punto blanco fijo sobre el iris.
    let (hu, hv) = (du + IRIS_RADIUS * 0.34, dv - IRIS_RADIUS * 0.36);
    let glint = 1.0 - smoothstep(0.012, 0.022, (hu * hu + hv * hv).sqrt());
    let globe = mix(globe, color(1.0, 1.0, 1.0), glint);

    // Párpado cerrado: un tono apenas más claro que la pared de la tormenta.
    // Se oscurece hacia el centro, donde se juntan las pestañas.
    let eyelid = color(0.17, 0.30, 0.13) * smoothstep(0.0, 0.05, v.abs()).max(0.12);
    let lidded = mix(mix(color(0.02, 0.035, 0.015), eyelid, shape.almond), globe, shape.opening);
    (lidded, shape.socket.max(shape.almond))
}

/// Ojo del lado oscuro: esclerótica enferma con venas, iris rojo encendido y
/// pupila de reptil que sigue a la cámara. Parpadea lento y a destiempo. Tiene luz
/// propia. Devuelve su luz, y cuánto tapa lo que hay debajo.
fn night_eye(
    noise: &OpenSimplex,
    p: Vector3,
    u: f32,
    v: f32,
    gaze: (f32, f32),
    time: f32,
) -> (Vector3, f32) {
    let open = blink(time, NIGHT_BLINK_PERIOD, NIGHT_BLINK_LENGTH)
        .min(blink(time, NIGHT_SECOND_BLINK_PERIOD, NIGHT_SECOND_BLINK_LENGTH));
    let shape = eye_shape(u, v, open);

    // Resplandor: se extiende más allá de la cuenca y se ve aun con el ojo cerrado.
    let halo_distance = (u * u + v * v).sqrt();
    let halo = (1.0 - smoothstep(0.08, 0.50, halo_distance)) * NIGHT_HALO * (0.35 + open * 0.65);
    let halo = color(1.0, 0.04, 0.02) * halo;
    if shape.socket <= 0.0 {
        return (halo, 0.0);
    }

    let du = u - gaze.0 * NIGHT_GAZE_REACH;
    let dv = v - gaze.1 * NIGHT_GAZE_REACH * 0.5;
    let radius = (du * du + dv * dv).sqrt() / IRIS_RADIUS;
    let angle = dv.atan2(du);

    // Venas: donde el ruido cruza por 0 se forman líneas que se ramifican.
    let vein = 1.0 - smoothstep(0.012, 0.05, sample(noise, p * 24.0).abs());
    let sclera = mix(color(0.24, 0.10, 0.05), color(0.80, 0.03, 0.02), vein);

    let fibers = (angle * 26.0).sin() * 0.5 + 0.5;
    let iris = mix(color(1.0, 0.13, 0.03), color(0.32, 0.0, 0.015), smoothstep(0.2, 1.0, radius));
    let iris = iris * (0.75 + fibers * 0.35);
    let globe = mix(iris, sclera, smoothstep(0.96, 1.04, radius));

    // Pupila vertical: se angosta hacia arriba y hacia abajo. Late despacio.
    let width = IRIS_RADIUS * (0.15 + (time * 1.3).sin() * 0.04);
    let along = (dv / (IRIS_RADIUS * 0.92)).abs();
    let slit = du.abs() - width * (1.0 - along * along).max(0.0);
    let globe = mix(globe, color(0.0, 0.0, 0.0), (1.0 - smoothstep(-0.002, 0.004, slit)) * (1.0 - smoothstep(0.98, 1.02, along)));

    let lit = mix(color(0.015, 0.0, 0.0), globe, shape.opening);
    (lit * shape.almond + halo, shape.socket.max(shape.almond))
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
    let day_center = DAY_EYE_CENTER.normalize();
    let night_center = NIGHT_EYE_CENTER.normalize();

    // Se calculan las bandas en el punto girado: los ojos no están encima de un
    // dibujo, sino que el patrón que ya existe se curva a su alrededor.
    let swirled = swirl(swirl(p, day_center), night_center);
    let atmosphere = band_color(band_value(noise, swirled, time));

    // Vetas de ceniza: ruido muy estirado que apaga parte de las bandas.
    let streak_point = color(swirled.x * 1.6, swirled.y * 8.0, swirled.z * 1.6);
    let streaks = smoothstep(0.50, 0.68, fbm(noise, streak_point + color(time * 0.03, 40.0, 0.0), 3));
    let mut surface = atmosphere * (1.0 - streaks * ASH_AMOUNT);
    let mut emission = Vector3::zero();

    if p.dot(day_center) > 0.5 {
        let (right, up) = eye_axes(day_center);
        let (eye, mask) = day_eye(p.dot(right), p.dot(up), time);
        surface = mix(surface, eye, mask);
        emission += eye * mask * DAY_EYE_GLOW;
    }

    if p.dot(night_center) > 0.5 {
        let (right, up) = eye_axes(night_center);
        // Dirección hacia la cámara en el plano del ojo: el iris se corre hacia ahí.
        let to_camera = (uniforms.camera_position - night_center).normalize();
        let gaze = (to_camera.dot(right), to_camera.dot(up));
        let (light, mask) = night_eye(noise, p, p.dot(right), p.dot(up), gaze, time);
        surface = surface * (1.0 - mask);
        emission += light;
    }

    surface * (0.1 + fragment.intensity * 0.9) + emission
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eyes_are_on_opposite_sides_of_the_light() {
        // La misma luz que usa `Uniforms`.
        let light = color(0.6, 0.5, 0.7).normalize();
        assert!(DAY_EYE_CENTER.normalize().dot(light) > 0.3);
        assert!(NIGHT_EYE_CENTER.normalize().dot(light) < -0.3);
    }

    #[test]
    fn swirl_leaves_points_outside_the_storm_untouched() {
        let center = DAY_EYE_CENTER.normalize();
        let far = center * -1.0;
        assert!((swirl(far, center) - far).length() < 1e-6);
    }

    #[test]
    fn swirl_rotates_inside_keeping_the_distance_to_the_center() {
        let center = DAY_EYE_CENTER.normalize();
        let near = (center + color(0.15, 0.05, 0.0)).normalize();
        let swirled = swirl(near, center);

        assert!((swirled - near).length() > 0.05);
        assert!((swirled.dot(center) - near.dot(center)).abs() < 1e-4);
    }

    /// El giro debe llegar a 0 en el borde: si no, las bandas tendrían un corte.
    #[test]
    fn swirl_is_continuous_at_the_storm_edge() {
        let center = DAY_EYE_CENTER.normalize();
        let side = center.cross(color(0.0, 1.0, 0.0)).normalize();
        let at = |angle: f32| center * angle.cos() + side * angle.sin();
        let just_inside = at(VORTEX_RADIUS - 0.001);
        assert!((swirl(just_inside, center) - just_inside).length() < 1e-3);
    }

    #[test]
    fn blink_closes_once_per_period_and_stays_open_otherwise() {
        assert_eq!(blink(DAY_BLINK_LENGTH + 0.1, DAY_BLINK_PERIOD, DAY_BLINK_LENGTH), 1.0);
        assert!(blink(DAY_BLINK_LENGTH * 0.5, DAY_BLINK_PERIOD, DAY_BLINK_LENGTH) < 0.01);
    }

    #[test]
    fn closed_eye_hides_the_globe_but_keeps_the_almond() {
        let open = eye_shape(0.0, 0.05, 1.0);
        let closed = eye_shape(0.0, 0.05, 0.0);
        assert!(open.opening > 0.99);
        assert!(closed.opening < 0.01);
        assert!(closed.almond > 0.99);
    }

    #[test]
    fn eye_shape_ends_outside_the_almond() {
        let far = eye_shape(EYE_WIDTH * 2.0, 0.0, 1.0);
        assert_eq!(far.almond, 0.0);
        assert_eq!(far.socket, 0.0);
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
