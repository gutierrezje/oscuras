use super::{camera::Camera, data_types};
use cgmath::{Matrix, Matrix4, Transform};

#[repr(C)]
pub struct Scene {
    //pub camera: Camera,
    pub geometry: Vec<data_types::Geometry>,
}

impl Scene {
    pub fn new() -> Self {
        let translate = cgmath::vec3(0f32, 0f32, 1f32);
        let t_mat = Matrix4::from_translation(translate);
        let s_mat = Matrix4::from_scale(1f32);
        let mut geometry = Vec::<data_types::Geometry>::new();
        let transf = s_mat * t_mat;
        let inverse = transf.inverse_transform().unwrap();
        let transp_inv = inverse.transpose();
        geometry.push(data_types::Geometry {
            transf: transf.into(),
            inverse: inverse.into(),
            transp_inv: transp_inv.into(),
            ty: data_types::GeomType::SPHERE,
        });
        Self {geometry}
    }
}
