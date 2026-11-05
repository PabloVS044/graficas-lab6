mod background;
mod camera;
mod fragment;
mod framebuffer;
mod obj;
mod pipeline;
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
use shaders::Body;
use vertex::Vertex;

const WIDTH: usize = 800;
const HEIGHT: usize = 600;
const MODEL_PATH: &str = "sphere.obj";
const FIELD_OF_VIEW: f32 = 45.0;
const NOISE_SEED: u32 = 7;
const CAPTURES_PATH: &str = "docs";

/// Valores que no cambian entre los vértices y fragmentos de un mismo cuadro.
pub struct Uniforms {
    pub model: Matrix,
    pub view: Matrix,
    pub projection: Matrix,
    pub width: usize,
    pub height: usize,
    #[allow(dead_code)] // Lo usarán los reflejos y el parallax.
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
}

/// Mismo orden que en clase, pero separando la prueba de profundidad del
/// sombreado. Así los shaders corren en paralelo: uno por núcleo del CPU.
fn render(
    framebuffer: &mut Framebuffer,
    background: &Background,
    vertices: &[Vertex],
    uniforms: &Uniforms,
) {
    background.paint(framebuffer, uniforms);

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

    let args: Vec<String> = std::env::args().collect();
    let has_flag = |flag: &str| args.iter().any(|arg| arg == flag);

    if has_flag("--export-images") {
        export_images(&background, &vertices, &noise);
        return;
    }
    if has_flag("--benchmark") {
        benchmark(&background, &vertices, &noise);
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
        render(&mut framebuffer, &background, &vertices, &uniforms);
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

/// Guarda una captura de cada cuerpo en `docs/`, sin abrir la ventana.
fn export_images(background: &Background, vertices: &[Vertex], noise: &OpenSimplex) {
    std::fs::create_dir_all(CAPTURES_PATH).expect("No se pudo crear la carpeta de capturas");

    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    for body in Body::ALL {
        let uniforms = Uniforms::new(&OrbitCamera::new(), 0.0, noise, body);
        render(&mut framebuffer, background, vertices, &uniforms);
        framebuffer.export(&format!("{CAPTURES_PATH}/{}.png", body.file_name()));
    }
}

/// Mide cuánto tarda un cuadro de cada cuerpo, sin abrir la ventana.
fn benchmark(background: &Background, vertices: &[Vertex], noise: &OpenSimplex) {
    const FRAMES: u32 = 60;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);

    for body in Body::ALL {
        let start = std::time::Instant::now();
        for frame in 0..FRAMES {
            let uniforms = Uniforms::new(&OrbitCamera::new(), frame as f32 / 60.0, noise, body);
            render(&mut framebuffer, background, vertices, &uniforms);
        }
        let milliseconds = start.elapsed().as_secs_f32() * 1000.0 / FRAMES as f32;
        println!("{}: {milliseconds:.1} ms por cuadro", body.title());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La esfera es convexa: en cada fila, los píxeles entre el primero y el
    /// último que la cubren también deben estar cubiertos.
    #[test]
    fn sphere_has_no_holes_from_any_angle() {
        let vertices = Obj::load(MODEL_PATH).unwrap().vertex_array();
        let noise = OpenSimplex::new(NOISE_SEED);
        let background = Background::new(&noise);
        let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);

        for step in 0..200 {
            let camera = OrbitCamera {
                yaw: step as f32 * 0.137,
                pitch: (step as f32 * 0.31).sin() * 1.4,
                distance: 2.0 + (step % 5) as f32 * 0.5,
            };
            let uniforms = Uniforms::new(&camera, 0.0, &noise, Body::Toxic);
            render(&mut framebuffer, &background, &vertices, &uniforms);

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
