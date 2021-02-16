use std::f32::consts::PI;

use cgmath::Vector3;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera {
    resolution: [u32; 2],
    pixel_length: [f32; 2],
    position: [f32; 3],
    aspect_ratio: f32,
    look_at: [f32; 3], // Direction camera is facing
    fovx: f32,
    up: [f32; 3],
    fovy: f32,
    right: [f32; 3],
    _padding: u32,
    view_dir: [f32; 3], // Direction to camera from look_at
}

impl Camera {
    pub fn new(win_res: &winit::dpi::PhysicalSize<u32>) -> Self {
        let resolution = [win_res.width, win_res.height];
        let pos = Vector3::<f32>::new(0.0, 0.0, 0.0);
        let at = Vector3::<f32>::new(0.0, 0.0, 1.0);
        let view = at - pos;

        let mut up = Vector3::<f32>::new(0.0, 1.0, 0.0);
        let r = up.cross(at);
        up = at.cross(r);

        let aspect_ratio = resolution[0] as f32 / resolution[1] as f32;
        let fovy = 45f32;
        let y_scaled = (fovy * PI / 180f32).tan();
        let x_scaled = (y_scaled * win_res.width as f32) / win_res.height as f32;
        let fovx = (x_scaled.atan() * 180f32) / PI;
        let pixel_length: [f32; 2] = [
            (2.0 * x_scaled) / win_res.width as f32,
            (2.0 * y_scaled) / win_res.height as f32,
        ];

        Self {
            resolution,
            pixel_length,
            position: pos.into(),
            aspect_ratio,
            look_at: at.into(),
            fovx,
            up: up.into(),
            fovy,
            right: r.into(),
            _padding: 0,
            view_dir: view.into(),
        }
    }

    pub fn res_x(&self) -> u32 {
        self.resolution[0]
    }

    pub fn res_y(&self) -> u32 {
        self.resolution[1]
    }
}
