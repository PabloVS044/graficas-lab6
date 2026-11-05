mod background;
mod camera;
mod capture;
mod fragment;
mod framebuffer;
mod obj;
mod pipeline;
mod rings;
mod shaders;
mod triangle;
mod vertex;

use background::Background;
use camera::{CameraRays, OrbitCamera};
use framebuffer::Framebuffer;
use noise::OpenSimplex;
use obj::Obj;
use rayon::prelude::*;
use raylib::prelude::*;
use rings::Rings;
use shaders::Body;
use vertex::Vertex;

const WIDTH: usize = 800;
const HEIGHT: usize = 600;
const MODEL_PATH: &str = "sphere.obj";
const FIELD_OF_VIEW: f32 = 45.0;
const NOISE_SEED: u32 = 7;
/// Tamaño de la luna respecto a la esfera original.
const MOON_SCALE: f32 = 0.14;
/// Distancia de la luna al centro de la jungla.
const MOON_ORBIT_RADIUS: f32 = 1.15;
/// Dónde empieza su órbita, en radianes.
const MOON_ORBIT_START: f32 = 0.35;
/// Radianes por segundo que recorre en su órbita.
const MOON_ORBIT_SPEED: f32 = 0.25;
/// Inclinación de la órbita, en radianes: 0 la deja en el plano horizontal.
const MOON_ORBIT_TILT: f32 = 0.35;

/// Valores que no cambian entre los vértices y fragmentos de un mismo cuadro.
pub struct Uniforms {
    pub model: Matrix,
    pub view: Matrix,
    pub projection: Matrix,
    pub width: usize,
    pub height: usize,
    pub camera_position: Vector3,
    pub rays: CameraRays,
    /// Dirección normalizada que va del planeta hacia la luz.
    pub light_direction: Vector3,
    pub time: f32,
    pub noise: OpenSimplex,
    pub body: Body,
}

impl Uniforms {
    fn new(camera: &OrbitCamera, time: f32, noise: &OpenSimplex, body: Body) -> Self {
        let field_of_view = FIELD_OF_VIEW.to_radians();

        Uniforms {
            model: Matrix::identity(),
            view: camera.view_matrix(),
            projection: Matrix::perspective(
                field_of_view as f64,
                WIDTH as f64 / HEIGHT as f64,
                0.1,
                100.0,
            ),
            width: WIDTH,
            height: HEIGHT,
            camera_position: camera.position(),
            rays: camera.rays(field_of_view, WIDTH, HEIGHT),
            light_direction: Vector3::new(0.6, 0.5, 0.7).normalize(),
            time,
            noise: noise.clone(),
            body,
        }
    }

    /// Los mismos valores del cuadro, con la matriz y el shader de la luna.
    fn for_moon(&self) -> Self {
        Uniforms {
            model: moon_model(self.time),
            view: self.view,
            projection: self.projection,
            width: self.width,
            height: self.height,
            camera_position: self.camera_position,
            rays: self.rays,
            light_direction: self.light_direction,
            time: self.time,
            noise: self.noise.clone(),
            body: Body::Moon,
        }
    }
}

/// Matriz que pone a la esfera en la órbita de la luna. Siempre le muestra la misma
/// cara a la jungla, como nuestra luna a la Tierra.
fn moon_model(time: f32) -> Matrix {
    let angle = MOON_ORBIT_START + time * MOON_ORBIT_SPEED;
    let (sin, cos) = angle.sin_cos();
    let (tilt_sin, tilt_cos) = MOON_ORBIT_TILT.sin_cos();

    // Rota alrededor de `y`, escala y traslada.
    let mut matrix = Matrix::identity();
    matrix.m0 = MOON_SCALE * cos;
    matrix.m2 = -MOON_SCALE * sin;
    matrix.m5 = MOON_SCALE;
    matrix.m8 = MOON_SCALE * sin;
    matrix.m10 = MOON_SCALE * cos;
    matrix.m12 = MOON_ORBIT_RADIUS * cos;
    matrix.m13 = MOON_ORBIT_RADIUS * sin * tilt_sin;
    matrix.m14 = MOON_ORBIT_RADIUS * sin * tilt_cos;
    matrix
}

/// Mismo orden que en clase, pero separando la prueba de profundidad del
/// sombreado. Así los shaders corren en paralelo: uno por núcleo del CPU.
fn render(
    framebuffer: &mut Framebuffer,
    background: &Background,
    rings: &Rings,
    vertices: &[Vertex],
    uniforms: &Uniforms,
) {
    background.paint(framebuffer, uniforms);
    draw_sphere(framebuffer, vertices, uniforms);

    if uniforms.body == Body::Jungle {
        draw_sphere(framebuffer, vertices, &uniforms.for_moon());
    }
    if uniforms.body == Body::RedSun {
        rings.paint(framebuffer, uniforms);
    }
}

/// Dibuja la esfera con la matriz y el shader de `uniforms`. El búfer de profundidad
/// decide qué se ve cuando la luna pasa por delante o por detrás de la jungla.
fn draw_sphere(framebuffer: &mut Framebuffer, vertices: &[Vertex], uniforms: &Uniforms) {
    let transformed: Vec<_> = vertices
        .iter()
        .map(|vertex| pipeline::vertex_shader(vertex, uniforms))
        .collect();

    let fragments: Vec<_> = transformed
        .par_chunks_exact(3)
        .flat_map_iter(|triangle_vertices| triangle::triangle(triangle_vertices, uniforms))
        .collect();

    let mut visible = Vec::with_capacity(fragments.len());
    for fragment in fragments {
        let index =
            fragment.position.y as usize * framebuffer.width + fragment.position.x as usize;

        if fragment.depth < framebuffer.depths[index] {
            framebuffer.depths[index] = fragment.depth;
            visible.push((index, fragment));
        }
    }

    let shaded: Vec<_> = visible
        .par_iter()
        .map(|(index, fragment)| (*index, shaders::fragment_shader(fragment, uniforms)))
        .collect();

    for (index, color) in shaded {
        framebuffer.colors[index] = color;
    }
}

fn main() {
    let model = Obj::load(MODEL_PATH).unwrap_or_else(|err| panic!("{err}"));
    let vertices = model.vertex_array();
    let noise = OpenSimplex::new(NOISE_SEED);
    let background = Background::new(&noise);
    let rings = Rings::new();

    let args: Vec<String> = std::env::args().collect();
    let has_flag = |flag: &str| args.iter().any(|arg| arg == flag);

    if has_flag("--export-images") {
        capture::export_images(&background, &rings, &vertices, &noise);
        return;
    }
    if has_flag("--export-clips") {
        capture::export_clips(&background, &rings, &vertices, &noise);
        return;
    }
    if has_flag("--benchmark") {
        benchmark(&background, &rings, &vertices, &noise);
        return;
    }

    let (mut window, thread) = raylib::init()
        .size(WIDTH as i32, HEIGHT as i32)
        .title("Lab 6: Planetas y estrellas con shaders")
        .build();
    window.set_target_fps(60);

    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    framebuffer.init_texture(&mut window, &thread);

    let mut camera = OrbitCamera::new();
    let mut body = Body::Jungle;

    while !window.window_should_close() {
        if window.is_key_pressed(KeyboardKey::KEY_ONE) {
            body = Body::Jungle;
        }
        if window.is_key_pressed(KeyboardKey::KEY_TWO) {
            body = Body::Toxic;
        }
        if window.is_key_pressed(KeyboardKey::KEY_THREE) {
            body = Body::RedSun;
        }
        camera.update(&window, window.get_frame_time());

        let uniforms = Uniforms::new(&camera, window.get_time() as f32, &noise, body);
        render(&mut framebuffer, &background, &rings, &vertices, &uniforms);
        framebuffer.upload();

        let mut drawing = window.begin_drawing(&thread);
        framebuffer.draw(&mut drawing);
        drawing.draw_fps(10, 10);
        drawing.draw_text(body.title(), 10, 34, 20, Color::RAYWHITE);
        drawing.draw_text(
            "1/2/3: cuerpo   Flechas: orbitar   Rueda: zoom   Inicio: reiniciar",
            10,
            575,
            16,
            Color::LIGHTGRAY,
        );
    }
}

/// Mide cuánto tarda un cuadro de cada cuerpo, sin abrir la ventana.
fn benchmark(background: &Background, rings: &Rings, vertices: &[Vertex], noise: &OpenSimplex) {
    const FRAMES: u32 = 60;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);

    for body in Body::ALL {
        let start = std::time::Instant::now();
        for frame in 0..FRAMES {
            let uniforms = Uniforms::new(&OrbitCamera::new(), frame as f32 / 60.0, noise, body);
            render(&mut framebuffer, background, rings, vertices, &uniforms);
        }
        let milliseconds = start.elapsed().as_secs_f32() * 1000.0 / FRAMES as f32;
        println!("{}: {milliseconds:.1} ms por cuadro", body.title());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Con la cámara lo más cerca posible, la luna puede pasarle muy cerca.
    #[test]
    fn moon_orbit_clears_the_planet_and_renders_up_close() {
        let vertices = Obj::load(MODEL_PATH).unwrap().vertex_array();
        let noise = OpenSimplex::new(NOISE_SEED);
        let background = Background::new(&noise);
        let rings = Rings::new();
        let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);

        for step in 0..120 {
            let time = step as f32 * 0.5;
            let model = moon_model(time);
            let center = Vector3::new(model.m12, model.m13, model.m14);
            assert!(center.length() - MOON_SCALE * 0.87 > 0.87, "choca con la jungla");

            let camera = OrbitCamera { yaw: step as f32 * 0.21, pitch: 0.3, distance: 1.3 };
            let uniforms = Uniforms::new(&camera, time, &noise, Body::Jungle);
            render(&mut framebuffer, &background, &rings, &vertices, &uniforms);
        }
    }

    /// La esfera es convexa: en cada fila, los píxeles entre el primero y el
    /// último que la cubren también deben estar cubiertos.
    #[test]
    fn sphere_has_no_holes_from_any_angle() {
        let vertices = Obj::load(MODEL_PATH).unwrap().vertex_array();
        let noise = OpenSimplex::new(NOISE_SEED);
        let background = Background::new(&noise);
        let rings = Rings::new();
        let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);

        for step in 0..200 {
            let camera = OrbitCamera {
                yaw: step as f32 * 0.137,
                pitch: (step as f32 * 0.31).sin() * 1.4,
                distance: 2.0 + (step % 5) as f32 * 0.5,
            };
            let uniforms = Uniforms::new(&camera, 0.0, &noise, Body::Toxic);
            render(&mut framebuffer, &background, &rings, &vertices, &uniforms);

            for row in framebuffer.depths.chunks_exact(WIDTH) {
                let covered = |depth: &f32| depth.is_finite();
                if let (Some(first), Some(last)) =
                    (row.iter().position(covered), row.iter().rposition(covered))
                {
                    assert!(row[first..=last].iter().all(covered), "hueco en el paso {step}");
                }
            }
        }
    }
}
