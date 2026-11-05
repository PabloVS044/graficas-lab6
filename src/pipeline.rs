use raylib::prelude::*;

use crate::Uniforms;
use crate::vertex::{TransformedVertex, Vertex};

/// Multiplica una matriz 4x4 por un vector en coordenadas homogéneas.
fn transform(matrix: &Matrix, v: Vector4) -> Vector4 {
    Vector4::new(
        matrix.m0 * v.x + matrix.m4 * v.y + matrix.m8 * v.z + matrix.m12 * v.w,
        matrix.m1 * v.x + matrix.m5 * v.y + matrix.m9 * v.z + matrix.m13 * v.w,
        matrix.m2 * v.x + matrix.m6 * v.y + matrix.m10 * v.z + matrix.m14 * v.w,
        matrix.m3 * v.x + matrix.m7 * v.y + matrix.m11 * v.z + matrix.m15 * v.w,
    )
}

/// objeto -> mundo -> cámara -> clip -> pantalla.
pub fn vertex_shader(vertex: &Vertex, uniforms: &Uniforms) -> TransformedVertex {
    let p = vertex.position;
    let world = transform(&uniforms.model, Vector4::new(p.x, p.y, p.z, 1.0));
    let clip = transform(&uniforms.projection, transform(&uniforms.view, world));

    // División de perspectiva.
    let inverse_w = 1.0 / clip.w;
    let ndc_x = clip.x * inverse_w;
    let ndc_y = clip.y * inverse_w;
    let ndc_z = clip.z * inverse_w;

    // w = 0 en un vector lo deja como dirección: la traslación no lo afecta.
    let n = vertex.normal;
    let normal = transform(&uniforms.model, Vector4::new(n.x, n.y, n.z, 0.0));

    TransformedVertex {
        screen: Vector3::new(
            (ndc_x + 1.0) * 0.5 * uniforms.width as f32,
            // En pantalla `y` crece hacia abajo.
            (1.0 - ndc_y) * 0.5 * uniforms.height as f32,
            ndc_z,
        ),
        inverse_w,
        object_position: p,
        world_position: Vector3::new(world.x, world.y, world.z),
        normal: Vector3::new(normal.x, normal.y, normal.z),
        uv: vertex.uv,
    }
}
