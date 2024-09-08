use benchlib::{bench_dataset, generate_vec, Generate, Rng};
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Generate for Vector3 {
    fn generate<R: Rng>(rand: &mut R) -> Self {
        Self {
            x: rand.gen(),
            y: rand.gen(),
            z: rand.gen(),
        }
    }
}

#[derive(Archive, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Triangle {
    pub v0: Vector3,
    pub v1: Vector3,
    pub v2: Vector3,
    pub normal: Vector3,
}

impl Generate for Triangle {
    fn generate<R: Rng>(rand: &mut R) -> Self {
        Self {
            v0: Vector3::generate(rand),
            v1: Vector3::generate(rand),
            v2: Vector3::generate(rand),
            normal: Vector3::generate(rand),
        }
    }
}

#[derive(
    rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, PartialEq,
)]
pub struct Mesh {
    pub triangles: Vec<Triangle>,
}

pub fn generate_mesh() -> Mesh {
    let mut rng = benchlib::rng();

    const TRIANGLES: usize = 125_000;
    Mesh {
        triangles: generate_vec::<_, Triangle>(
            &mut rng,
            TRIANGLES..TRIANGLES + 1,
        ),
    }
}

bench_dataset!(Mesh = generate_mesh());
