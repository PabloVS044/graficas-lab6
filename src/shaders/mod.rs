use raylib::prelude::*;

use crate::Uniforms;
use crate::fragment::Fragment;

/// Calcula el color de un fragmento. Por ahora, una esfera gris con la luz del
/// renderer para comprobar que el pipeline funciona.
pub fn fragment_shader(fragment: &Fragment, _uniforms: &Uniforms) -> Vector3 {
    Vector3::new(0.75, 0.75, 0.75) * (0.12 + fragment.intensity * 0.88)
}
