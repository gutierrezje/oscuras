use super::data_types;

#[repr(C)]
pub struct Scene {
    pub geometry: Vec<data_types::Sphere>,
}
