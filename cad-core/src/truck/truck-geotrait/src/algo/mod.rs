#![allow(clippy::many_single_char_names)]

use crate::traits::*;
use truck_base::{
    cgmath64::*,
    hash::HashGen,
    newton::{self, CalcOutput},
    tolerance::*,
};

pub mod curve;
pub mod surface;
