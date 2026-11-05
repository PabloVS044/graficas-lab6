use raylib::prelude::*;

const START_YAW: f32 = 0.0;
const START_PITCH: f32 = 0.0;
const START_DISTANCE: f32 = 2.6;

/// Calcula hacia dónde mira cada píxel de la ventana.
#[derive(Clone, Copy)]
pub struct CameraRays {
    forward: Vector3,
    right: Vector3,
    up: Vector3,
    width: f32,
    height: f32,
}

impl CameraRays {
    /// Dirección normalizada del rayo que sale de la cámara por el píxel.
    pub fn direction(&self, x: usize, y: usize) -> Vector3 {
        let ndc_x = (x as f32 + 0.5) / self.width * 2.0 - 1.0;
        let ndc_y = 1.0 - (y as f32 + 0.5) / self.height * 2.0;
        (self.forward + self.right * ndc_x + self.up * ndc_y).normalize()
    }
}

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

    pub fn rays(&self, field_of_view: f32, width: usize, height: usize) -> CameraRays {
        let forward = (Vector3::zero() - self.position()).normalize();
        let right = forward.cross(Vector3::new(0.0, 1.0, 0.0)).normalize();
        let up = right.cross(forward);
        let half_height = (field_of_view * 0.5).tan();
        let half_width = half_height * width as f32 / height as f32;

        CameraRays {
            forward,
            right: right * half_width,
            up: up * half_height,
            width: width as f32,
            height: height as f32,
        }
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
