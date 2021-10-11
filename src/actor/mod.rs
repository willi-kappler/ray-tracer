use crate::float::Float;
use crate::hitable::Hitable;
use crate::material::Material;

pub struct Actor<T>
    where T: Float
{
    pub hitable: Box<dyn Hitable<T>>,
    pub material: Box<dyn Material<T>>
}
