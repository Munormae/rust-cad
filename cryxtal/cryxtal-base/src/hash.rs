#![allow(missing_docs)]

use cgmath::num_traits::{Float, FromPrimitive};
use cgmath::{Point1, Point2, Point3, Vector1, Vector2, Vector3, Vector4};

const STREAM_INCREMENT: u64 = 0x9e37_79b9_7f4a_7c15;
const MIX_MULT_A: u64 = 0xbf58_476d_1ce4_e5b9;
const MIX_MULT_B: u64 = 0x94d0_49bb_1331_11eb;
const LANE_SALT: u64 = 0x632b_e59b_d9b4_e019;
const SEED_SALT: u64 = 0x27bb_2ee6_87b0_b0fd;

#[inline]
fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(MIX_MULT_A);
    z = (z ^ (z >> 27)).wrapping_mul(MIX_MULT_B);
    z ^ (z >> 31)
}

#[derive(Clone, Copy)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    #[inline]
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    #[inline]
    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(STREAM_INCREMENT);
        mix64(self.state)
    }
}

#[inline]
fn scalar_seed<S: Float>(value: S, lane: u64) -> u64 {
    let bits = value
        .to_f64()
        .unwrap_or(0.0)
        .to_bits()
        .wrapping_add((lane + 1).wrapping_mul(LANE_SALT));
    mix64(bits)
}

#[inline]
fn slice_seed<S: Float, const N: usize>(values: &[S; N]) -> u64 {
    values
        .iter()
        .enumerate()
        .fold(mix64(SEED_SALT ^ N as u64), |acc, (idx, &value)| {
            mix64(acc ^ scalar_seed(value, idx as u64))
        })
}

#[inline]
fn to_unit<S: Float + FromPrimitive>(value: u64) -> S {
    const INV_U64: f64 = 1.0 / (u64::MAX as f64 + 1.0);
    let f = (value as f64) * INV_U64;
    S::from_f64(f).unwrap()
}

fn hash_scalar_channels<S: Float + FromPrimitive, const N: usize>(value: S) -> [S; N] {
    let mut stream = SplitMix64::new(scalar_seed(value, 0));
    std::array::from_fn(|_| to_unit::<S>(stream.next()))
}

fn hash_array_channels<S: Float + FromPrimitive, const LEN: usize, const N: usize>(
    values: &[S; LEN],
) -> [S; N] {
    let mut stream = SplitMix64::new(slice_seed(values));
    std::array::from_fn(|_| to_unit::<S>(stream.next()))
}

/// Интерфейс генерации детерминированных псевдослучайных значений из семени произвольного типа.
pub trait HashGen<S> {
    fn hash1(gen: Self) -> S;
    fn hash2(gen: Self) -> [S; 2];
    fn hash3(gen: Self) -> [S; 3];
    fn hash4(gen: Self) -> [S; 4];
}

impl<S: Float + FromPrimitive> HashGen<S> for S {
    fn hash1(gen: Self) -> S {
        hash_scalar_channels::<S, 1>(gen)[0]
    }

    fn hash2(gen: Self) -> [S; 2] {
        hash_scalar_channels::<S, 2>(gen)
    }

    fn hash3(gen: Self) -> [S; 3] {
        hash_scalar_channels::<S, 3>(gen)
    }

    fn hash4(gen: Self) -> [S; 4] {
        hash_scalar_channels::<S, 4>(gen)
    }
}

impl<S: Float + FromPrimitive> HashGen<S> for [S; 1] {
    fn hash1(gen: Self) -> S {
        hash_array_channels::<S, 1, 1>(&gen)[0]
    }

    fn hash2(gen: Self) -> [S; 2] {
        hash_array_channels::<S, 1, 2>(&gen)
    }

    fn hash3(gen: Self) -> [S; 3] {
        hash_array_channels::<S, 1, 3>(&gen)
    }

    fn hash4(gen: Self) -> [S; 4] {
        hash_array_channels::<S, 1, 4>(&gen)
    }
}

impl<S: Float + FromPrimitive> HashGen<S> for [S; 2] {
    fn hash1(gen: Self) -> S {
        hash_array_channels::<S, 2, 1>(&gen)[0]
    }

    fn hash2(gen: Self) -> [S; 2] {
        hash_array_channels::<S, 2, 2>(&gen)
    }

    fn hash3(gen: Self) -> [S; 3] {
        hash_array_channels::<S, 2, 3>(&gen)
    }

    fn hash4(gen: Self) -> [S; 4] {
        hash_array_channels::<S, 2, 4>(&gen)
    }
}

impl<S: Float + FromPrimitive> HashGen<S> for [S; 3] {
    fn hash1(gen: Self) -> S {
        hash_array_channels::<S, 3, 1>(&gen)[0]
    }

    fn hash2(gen: Self) -> [S; 2] {
        hash_array_channels::<S, 3, 2>(&gen)
    }

    fn hash3(gen: Self) -> [S; 3] {
        hash_array_channels::<S, 3, 3>(&gen)
    }

    fn hash4(gen: Self) -> [S; 4] {
        hash_array_channels::<S, 3, 4>(&gen)
    }
}

impl<S: Float + FromPrimitive> HashGen<S> for [S; 4] {
    fn hash1(gen: Self) -> S {
        hash_array_channels::<S, 4, 1>(&gen)[0]
    }

    fn hash2(gen: Self) -> [S; 2] {
        hash_array_channels::<S, 4, 2>(&gen)
    }

    fn hash3(gen: Self) -> [S; 3] {
        hash_array_channels::<S, 4, 3>(&gen)
    }

    fn hash4(gen: Self) -> [S; 4] {
        hash_array_channels::<S, 4, 4>(&gen)
    }
}

macro_rules! derive_hashgen {
    ($from: ty, $into: ty) => {
        impl<S: Float + FromPrimitive> HashGen<S> for $from {
            fn hash1(gen: Self) -> S {
                <$into>::hash1(gen.into())
            }

            fn hash2(gen: Self) -> [S; 2] {
                <$into>::hash2(gen.into())
            }

            fn hash3(gen: Self) -> [S; 3] {
                <$into>::hash3(gen.into())
            }

            fn hash4(gen: Self) -> [S; 4] {
                <$into>::hash4(gen.into())
            }
        }
    };
}

derive_hashgen!(Point1<S>, [S; 1]);
derive_hashgen!(Point2<S>, [S; 2]);
derive_hashgen!(Point3<S>, [S; 3]);
derive_hashgen!(Vector1<S>, [S; 1]);
derive_hashgen!(Vector2<S>, [S; 2]);
derive_hashgen!(Vector3<S>, [S; 3]);
derive_hashgen!(Vector4<S>, [S; 4]);

/// Преобразует генератор в равномерно распределённую точку на единичной сфере.
pub fn take_one_unit<G: HashGen<f64>>(gen: G) -> Vector3<f64> {
    let u = HashGen::hash2(gen);
    let theta = 2.0 * std::f64::consts::PI * u[0];
    let z = 2.0 * u[1] - 1.0;
    let r = f64::sqrt(1.0 - z * z);
    Vector3::new(r * f64::cos(theta), r * f64::sin(theta), z)
}
