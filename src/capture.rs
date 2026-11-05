//! Genera las capturas y los clips que muestra el README, sin abrir la ventana.
//! Los clips quedan como secuencias de PNG; `scripts/make_gifs.sh` los vuelve GIF.

use noise::OpenSimplex;
use raylib::prelude::*;

use crate::background::Background;
use crate::camera::OrbitCamera;
use crate::framebuffer::Framebuffer;
use crate::rings::Rings;
use crate::shaders::Body;
use crate::vertex::Vertex;
use crate::{HEIGHT, Uniforms, WIDTH, render};

const OUTPUT_PATH: &str = "docs";
const FRAMES_PER_SECOND: u32 = 15;
const DISTANCE: f32 = 2.6;

/// Una imagen fija.
struct Still {
    name: &'static str,
    body: Body,
    time: f32,
    yaw: f32,
    pitch: f32,
}

const STILLS: [Still; 5] = [
    Still { name: "jungla", body: Body::Jungle, time: 0.0, yaw: 0.0, pitch: 0.0 },
    Still { name: "jungla_noche", body: Body::Jungle, time: 2.0, yaw: -1.9, pitch: 0.0 },
    Still { name: "toxico", body: Body::Toxic, time: 0.0, yaw: 0.0, pitch: 0.0 },
    Still { name: "toxico_noche", body: Body::Toxic, time: 2.0, yaw: -2.54, pitch: -0.25 },
    Still { name: "sol", body: Body::RedSun, time: 7.0, yaw: 0.0, pitch: 0.0 },
];

/// Un clip: de cada par de valores, el primero es el del inicio y el segundo el
/// del final. `time` puede avanzar más rápido que el video para que un movimiento
/// lento se aprecie en pocos segundos.
struct Clip {
    name: &'static str,
    body: Body,
    seconds: u32,
    time: (f32, f32),
    yaw: (f32, f32),
    pitch: f32,
    /// Giro de la luz alrededor del planeta, en radianes.
    light_turn: (f32, f32),
}

const CLIPS: [Clip; 7] = [
    // Tiempo a 6x: las nubes giran despacio.
    Clip {
        name: "jungla_nubes",
        body: Body::Jungle,
        seconds: 5,
        time: (0.0, 30.0),
        yaw: (0.0, 0.0),
        pitch: 0.0,
        light_turn: (0.0, 0.0),
    },
    // Nubes quietas y luz en movimiento: solo pueden cambiar las sombras.
    Clip {
        name: "jungla_sombras",
        body: Body::Jungle,
        seconds: 5,
        time: (0.0, 0.0),
        yaw: (0.0, 0.0),
        pitch: 0.0,
        light_turn: (-0.9, 0.9),
    },
    // Tiempo quieto y cámara en movimiento: el reflejo del agua se desplaza.
    Clip {
        name: "jungla_reflejo",
        body: Body::Jungle,
        seconds: 5,
        time: (0.0, 0.0),
        yaw: (0.25, 1.25),
        pitch: 0.1,
        light_turn: (0.0, 0.0),
    },
    Clip {
        name: "jungla_noche",
        body: Body::Jungle,
        seconds: 6,
        time: (0.0, 6.0),
        yaw: (0.0, -2.2),
        pitch: 0.0,
        light_turn: (0.0, 0.0),
    },
    // Tiempo a 5x: las bandas fluyen despacio.
    Clip {
        name: "toxico_tormenta",
        body: Body::Toxic,
        seconds: 5,
        time: (0.0, 25.0),
        yaw: (0.0, 0.0),
        pitch: 0.0,
        light_turn: (0.0, 0.0),
    },
    Clip {
        name: "toxico_ondas",
        body: Body::Toxic,
        seconds: 7,
        time: (0.0, 7.0),
        yaw: (-2.54, -2.54),
        pitch: -0.25,
        light_turn: (0.0, 0.0),
    },
    // Tiempo a 3x: el plasma y los anillos.
    Clip {
        name: "sol",
        body: Body::RedSun,
        seconds: 6,
        time: (0.0, 18.0),
        yaw: (0.0, 0.0),
        pitch: 0.0,
        light_turn: (0.0, 0.0),
    },
];

fn lerp(range: (f32, f32), amount: f32) -> f32 {
    range.0 + (range.1 - range.0) * amount
}

/// Gira una dirección alrededor del eje `y`.
fn turned(direction: Vector3, angle: f32) -> Vector3 {
    let (sin, cos) = angle.sin_cos();
    Vector3::new(
        direction.x * cos + direction.z * sin,
        direction.y,
        direction.z * cos - direction.x * sin,
    )
}

pub fn export_images(background: &Background, rings: &Rings, vertices: &[Vertex], noise: &OpenSimplex) {
    std::fs::create_dir_all(OUTPUT_PATH).expect("No se pudo crear la carpeta de capturas");
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);

    for still in &STILLS {
        let camera = OrbitCamera { yaw: still.yaw, pitch: still.pitch, distance: DISTANCE };
        let uniforms = Uniforms::new(&camera, still.time, noise, still.body);
        render(&mut framebuffer, background, rings, vertices, &uniforms);
        framebuffer.export(&format!("{OUTPUT_PATH}/{}.png", still.name));
    }
}

pub fn export_clips(background: &Background, rings: &Rings, vertices: &[Vertex], noise: &OpenSimplex) {
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);

    for clip in &CLIPS {
        let folder = format!("{OUTPUT_PATH}/frames/{}", clip.name);
        std::fs::create_dir_all(&folder).expect("No se pudo crear la carpeta de cuadros");

        let frames = clip.seconds * FRAMES_PER_SECOND;
        for frame in 0..frames {
            let amount = frame as f32 / (frames - 1) as f32;
            let camera = OrbitCamera {
                yaw: lerp(clip.yaw, amount),
                pitch: clip.pitch,
                distance: DISTANCE,
            };

            let mut uniforms = Uniforms::new(&camera, lerp(clip.time, amount), noise, clip.body);
            uniforms.light_direction = turned(uniforms.light_direction, lerp(clip.light_turn, amount));

            render(&mut framebuffer, background, rings, vertices, &uniforms);
            framebuffer.export(&format!("{folder}/{frame:04}.png"));
        }
    }
}
