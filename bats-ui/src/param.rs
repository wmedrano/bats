use std::fmt::{Display, Formatter};

use bats_lib::plugin::metadata::ParamType;

/// A formatter for params.
pub struct ParamFormatter {
    /// The param type.
    param_type: ParamType,
    /// The value of the param.
    value: f32,
}

/// Create a ParamFormatter from a `ParamType` and its value.
impl From<(ParamType, f32)> for ParamFormatter {
    /// Create a ParamFormatter from a `ParamType` and its value.
    fn from(v: (ParamType, f32)) -> ParamFormatter {
        ParamFormatter {
            param_type: v.0,
            value: v.1,
        }
    }
}

/// Create human readable format for the parameter.
impl Display for ParamFormatter {
    /// Create human readable format for the parameter.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.param_type {
            ParamType::Float => write!(f, "{}", self.value),
            ParamType::Bool => write!(f, "{}", if self.value < 0.5 { "off" } else { "on" }),
            ParamType::Percent => write!(f, "{:.1}%", self.value * 100.0),
        }
    }
}
