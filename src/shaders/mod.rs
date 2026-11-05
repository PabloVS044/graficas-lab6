mod jungle;
pub mod noise_helpers;
mod sun;
mod toxic;

use raylib::prelude::*;

use crate::Uniforms;
use crate::fragment::Fragment;

/// Los cuerpos celestes. Todos usan la misma esfera.
#[derive(Clone, Copy, PartialEq)]
pub enum Body {
    Jungle,
    Toxic,
    RedSun,
}

impl Body {
    pub const ALL: [Body; 3] = [Body::Jungle, Body::Toxic, Body::RedSun];

    /// Nombre para mostrar en la ventana.
    pub fn title(self) -> &'static str {
        match self {
            Body::Jungle => "Jungla",
            Body::Toxic => "Mundo toxico",
            Body::RedSun => "Sol rojo",
        }
    }

    /// Nombre del archivo de su captura.
    pub fn file_name(self) -> &'static str {
        match self {
            Body::Jungle => "jungla",
            Body::Toxic => "toxico",
            Body::RedSun => "sol",
        }
    }
}

/// Atajo para escribir un color (o un desplazamiento) con tres componentes.
pub fn color(r: f32, g: f32, b: f32) -> Vector3 {
    Vector3::new(r, g, b)
}

/// Interpolación lineal entre dos colores.
pub fn mix(a: Vector3, b: Vector3, amount: f32) -> Vector3 {
    a * (1.0 - amount) + b * amount
}

/// 0 antes de `start`, 1 después de `end` y una transición suave entre ambos.
pub fn smoothstep(start: f32, end: f32, value: f32) -> f32 {
    let t = ((value - start) / (end - start)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Elige el shader del cuerpo activo.
pub fn fragment_shader(fragment: &Fragment, uniforms: &Uniforms) -> Vector3 {
    match uniforms.body {
        Body::Jungle => jungle::shade(fragment, uniforms),
        Body::RedSun => sun::shade(fragment, uniforms),
        Body::Toxic => toxic::shade(fragment, uniforms),
    }
}
