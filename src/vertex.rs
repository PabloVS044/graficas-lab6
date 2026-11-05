use raylib::math::{Vector2, Vector3};

/// Vértice tal como viene del OBJ. Es la misma geometría para los tres cuerpos.
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: Vector3,
    pub normal: Vector3,
    pub uv: Vector2,
}

/// Lo que devuelve el vertex shader: el vértice ya en pantalla, junto con los
/// datos que el rasterizador interpola para cada fragmento.
#[derive(Clone, Copy, Debug)]
pub struct TransformedVertex {
    /// `x`, `y` en píxeles; `z` es la profundidad.
    pub screen: Vector3,
    /// `1 / w`: lo necesita la interpolación con corrección de perspectiva.
    pub inverse_w: f32,
    pub object_position: Vector3,
    pub world_position: Vector3,
    pub normal: Vector3,
    pub uv: Vector2,
}
