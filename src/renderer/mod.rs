use super::*;

#[derive(ugli::Vertex, Clone)]
pub struct Vertex {
    pub a_pos: Vec2<f32>,
}

pub struct Renderer {
    quad: ugli::VertexBuffer<Vertex>,
    white_texture: ugli::Texture,
    program: ugli::Program,
}

impl Renderer {
    pub fn new(geng: &Rc<Geng>) -> Self {
        Self {
            quad: ugli::VertexBuffer::new_static(
                geng.ugli(),
                vec![
                    Vertex {
                        a_pos: vec2(0.0, 0.0),
                    },
                    Vertex {
                        a_pos: vec2(1.0, 0.0),
                    },
                    Vertex {
                        a_pos: vec2(1.0, 1.0),
                    },
                    Vertex {
                        a_pos: vec2(0.0, 1.0),
                    },
                ],
            ),
            program: geng
                .shader_lib()
                .compile(include_str!("program.glsl"))
                .unwrap(),
            white_texture: ugli::Texture::new_with(geng.ugli(), vec2(1, 1), |_| Color::WHITE),
        }
    }
    pub fn draw(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        camera: &Camera,
        matrix: Mat4<f32>,
        texture: Option<&ugli::Texture>,
        color: Color<f32>,
    ) {
        let texture = texture.unwrap_or(&self.white_texture);
        let camera_uniforms = camera.uniforms(framebuffer.size().map(|x| x as f32));
        let uniforms = (
            camera_uniforms,
            ugli::uniforms! {
                u_model_matrix: matrix,
                u_texture: texture,
                u_color: color,
            },
        );
        ugli::draw(
            framebuffer,
            &self.program,
            ugli::DrawMode::TriangleFan,
            &self.quad,
            uniforms,
            ugli::DrawParameters {
                blend_mode: Some(default()),
                ..default()
            },
        );
    }
}
