use super::data;

#[repr(C)]
pub struct Scene {
    pub geometry: Vec<data::Sphere>,
}