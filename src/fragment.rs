use raylib::math::{Vector2, Vector3};

/// Todo lo que un fragment shader sabe sobre un píxel.
#[allow(dead_code)] // Los shaders de los planetas usarán todos los campos.
pub struct Fragment {
    /// Píxel de la ventana.
    pub position: Vector2,
    pub depth: f32,
    /// Posición sobre la esfera, antes de transformarla. Los patrones se
    /// calculan aquí para que no se muevan al mover la cámara.
    pub object_position: Vector3,
    pub world_position: Vector3,
    /// Normal interpolada y normalizada, en coordenadas de mundo.
    pub normal: Vector3,
    pub uv: Vector2,
    /// `normal · luz`, de 0 (no recibe luz) a 1 (la recibe de frente).
    pub intensity: f32,
}
