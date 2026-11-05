use raylib::prelude::*;

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub colors: Vec<Vector3>,
    pub depths: Vec<f32>,
    pixels: Vec<u8>,
    texture: Option<Texture2D>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Framebuffer {
            width,
            height,
            colors: vec![Vector3::zero(); width * height],
            depths: vec![f32::INFINITY; width * height],
            pixels: vec![0; width * height * 4],
            texture: None,
        }
    }

    pub fn init_texture(&mut self, window: &mut RaylibHandle, thread: &RaylibThread) {
        let image =
            Image::gen_image_color(self.width as i32, self.height as i32, Color::BLACK);
        self.texture = Some(window.load_texture_from_image(thread, &image).unwrap());
    }

    /// Copia los colores a la textura de la ventana.
    pub fn upload(&mut self) {
        for (color, pixel) in self.colors.iter().zip(self.pixels.chunks_exact_mut(4)) {
            let [r, g, b, a] = to_rgba(*color);
            pixel[0] = r;
            pixel[1] = g;
            pixel[2] = b;
            pixel[3] = a;
        }

        let texture = self
            .texture
            .as_mut()
            .expect("Falta llamar a init_texture antes de upload");
        texture.update_texture(&self.pixels).unwrap();
    }

    pub fn draw(&self, drawing: &mut RaylibDrawHandle) {
        if let Some(texture) = &self.texture {
            drawing.draw_texture(texture, 0, 0, Color::WHITE);
        }
    }

    /// Guarda la imagen como PNG, sin necesidad de abrir una ventana.
    pub fn export(&self, path: &str) {
        let mut image =
            Image::gen_image_color(self.width as i32, self.height as i32, Color::BLACK);

        for y in 0..self.height {
            for x in 0..self.width {
                let [r, g, b, a] = to_rgba(self.colors[y * self.width + x]);
                image.draw_pixel(x as i32, y as i32, Color::new(r, g, b, a));
            }
        }

        image.export_image(path);
    }
}

fn to_rgba(color: Vector3) -> [u8; 4] {
    [
        (color.x.clamp(0.0, 1.0) * 255.0) as u8,
        (color.y.clamp(0.0, 1.0) * 255.0) as u8,
        (color.z.clamp(0.0, 1.0) * 255.0) as u8,
        255,
    ]
}
