use std::fmt::Display;
use std::ops::Deref;

use bevy_math::{DVec2, DVec3};
use winnow::ascii::float;
use winnow::Parser;

use crate::parser::geometry::{p_vec2, p_vec3};
use crate::parser::{IFCParse, IFCParser};
use crate::prelude::*;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct RealPrimitive(pub f64);

impl IFCParse for RealPrimitive {
    fn parse<'a>() -> impl IFCParser<'a, Self>
    where
        Self: Sized,
    {
        float.map(Self)
    }
}

impl IfcVerify for RealPrimitive {}
impl IfcType for RealPrimitive {}

impl From<f64> for RealPrimitive {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl From<RealPrimitive> for f64 {
    fn from(val: RealPrimitive) -> Self {
        val.0
    }
}

impl Display for RealPrimitive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format_real_primitive(self.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IfcDVec2(pub(crate) DVec2);

impl IFCParse for IfcDVec2 {
    fn parse<'a>() -> impl IFCParser<'a, Self>
    where
        Self: Sized,
    {
        p_vec2().map(Self)
    }
}

impl Display for IfcDVec2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({x},{y})",
            x = format_real_primitive(self.0.x),
            y = format_real_primitive(self.0.y),
        )
    }
}

impl Deref for IfcDVec2 {
    type Target = DVec2;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<DVec2> for IfcDVec2 {
    fn from(value: DVec2) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IfcDVec3(pub(crate) DVec3);

impl IFCParse for IfcDVec3 {
    fn parse<'a>() -> impl IFCParser<'a, Self>
    where
        Self: Sized,
    {
        p_vec3().map(Self)
    }
}

impl Display for IfcDVec3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({x},{y},{z})",
            x = format_real_primitive(self.0.x),
            y = format_real_primitive(self.0.y),
            z = format_real_primitive(self.0.z),
        )
    }
}

impl Deref for IfcDVec3 {
    type Target = DVec3;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<DVec3> for IfcDVec3 {
    fn from(value: DVec3) -> Self {
        Self(value)
    }
}

pub(crate) fn format_real_primitive(d: f64) -> String {
    // might need tuning 10 decimals allowed
    let is_scientific = d
        .fract()
        .to_string()
        .chars()
        .filter(|c| c.is_ascii_digit() && *c != '0')
        .count()
        > 10;

    let fmt = if is_scientific {
        format_sci_double
    } else {
        format_non_sci_double
    };

    fmt(d)
}

pub(crate) fn format_sci_double(d: f64) -> String {
    format!("{0:.1$E}", d, 14)
}

pub(crate) fn format_non_sci_double(d: f64) -> String {
    format!(
        "{d}{opt_p}",
        opt_p = (d.fract() == 0.0).then_some(".").unwrap_or_default()
    )
}
