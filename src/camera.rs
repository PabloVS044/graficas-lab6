use raylib::prelude::*;

const START_YAW: f32 = 0.0;
const START_PITCH: f32 = 0.0;
const START_DISTANCE: f32 = 2.6;

/// Cámara que orbita alrededor del centro de la esfera.
pub struct OrbitCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl OrbitCamera {
    pub fn new() -> Self {
        OrbitCamera {
            yaw: START_YAW,
            pitch: START_PITCH,
            distance: START_DISTANCE,
        }
    }

    pub fn position(&self) -> Vector3 {
        Vector3::new(
            self.distance * self.pitch.cos() * self.yaw.sin(),
            self.distance * self.pitch.sin(),
            self.distance * self.pitch.cos() * self.yaw.cos(),
        )
    }

    pub fn view_matrix(&self) -> Matrix {
        Matrix::look_at(
            self.position(),
            Vector3::zero(),
            Vector3::new(0.0, 1.0, 0.0),
        )
    }

    /// Flechas: orbitar. Rueda del mouse: acercar. Inicio: reiniciar.
    pub fn update(&mut self, window: &RaylibHandle, delta: f32) {
        const SPEED: f32 = 1.5;

        if window.is_key_down(KeyboardKey::KEY_LEFT) {
            self.yaw -= SPEED * delta;
        }
        if window.is_key_down(KeyboardKey::KEY_RIGHT) {
            self.yaw += SPEED * delta;
        }
        if window.is_key_down(KeyboardKey::KEY_UP) {
            self.pitch += SPEED * delta;
        }
        if window.is_key_down(KeyboardKey::KEY_DOWN) {
            self.pitch -= SPEED * delta;
        }
        self.pitch = self.pitch.clamp(-1.5, 1.5);

        // Nunca entra a la esfera, cuyo radio máximo es 0.866.
        self.distance = (self.distance - window.get_mouse_wheel_move() * 0.2).clamp(1.3, 6.0);

        if window.is_key_pressed(KeyboardKey::KEY_HOME) {
            *self = OrbitCamera::new();
        }
    }
}
