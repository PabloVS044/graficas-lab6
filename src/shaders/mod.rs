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

/// Elige el shader del cuerpo activo. Pendientes: esfera gris hasta que tengan
/// su shader.
pub fn fragment_shader(fragment: &Fragment, uniforms: &Uniforms) -> Vector3 {
    match uniforms.body {
        Body::Jungle | Body::Toxic | Body::RedSun => {
            Vector3::new(0.75, 0.75, 0.75) * (0.12 + fragment.intensity * 0.88)
        }
    }
}
