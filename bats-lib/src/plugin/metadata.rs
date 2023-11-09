use std::fmt::{Display, Formatter};

/// Metadata for a plugin.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Metadata {
    /// The name.
    pub name: &'static str,
    /// The parameters.
    pub params: &'static [Param],
}

/// The type for the parameter.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ParamType {
    /// A floating point number.
    Float,
    /// A toggle where >=0.5 is on and <0.5 is off.
    Bool,
    /// A decibel value where 1.0 is 0 dB.
    Decibel,
    /// A percentage between 0% and 100%.
    Percent,
    /// A frequency represented in Hz or kHz.
    Frequency,
    /// A duration.
    Duration,
}

/// A single parameter.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Param {
    /// An id unique for the param in the plugin.
    pub id: u32,
    /// The name of the parameter.
    pub name: &'static str,
    /// The parameter type.
    pub param_type: ParamType,
    /// The default value for the param.
    pub default_value: f32,
    /// The minimum value.
    pub min_value: f32,
    /// The maximum value.
    pub max_value: f32,
}

impl Metadata {
    /// Get the parameter by name.
    pub fn param_by_name(&self, name: &str) -> Option<&Param> {
        self.params.iter().find(|p| p.name == name)
    }

    /// Get the paramter by id.
    pub fn param_by_id(&self, id: u32) -> Option<&Param> {
        self.params.iter().find(|p| p.id == id)
    }
}

impl Param {
    /// Return the value in a form that can be formatted for display.
    pub fn formatted(&self, value: f32) -> impl Display {
        self.param_type.formatted(value)
    }
}

impl ParamType {
    /// Return the value in a form that can be formatted for display.
    pub fn formatted(&self, value: f32) -> impl Display {
        ParamFormatter {
            param_type: *self,
            value,
        }
    }
}

/// A formatter for params.
struct ParamFormatter {
    /// The param type.
    param_type: ParamType,
    /// The value of the param.
    value: f32,
}

/// Create human readable format for the parameter.
impl Display for ParamFormatter {
    /// Create human readable format for the parameter.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.param_type {
            ParamType::Float => write!(f, "{}", self.value),
            ParamType::Bool => write!(f, "{}", if self.value < 0.5 { "off" } else { "on" }),
            ParamType::Decibel => {
                let db = 20.0 * self.value.log10();
                if db.abs() < 10.0 {
                    write!(f, "{db:.2} dB")
                } else {
                    write!(f, "{db:.1} dB")
                }
            }
            ParamType::Percent => write!(f, "{:.1}%", self.value * 100.0),
            ParamType::Frequency => {
                if self.value < 1000.0 {
                    write!(f, "{freq:.0} Hz", freq = self.value)
                } else {
                    write!(f, "{k_freq:.2} kHz", k_freq = self.value / 1000.0)
                }
            }
            ParamType::Duration => match self.value {
                v if v < 0.01 => write!(f, "{msec:.1}ms", msec = v * 1000.0),
                v if v < 1.0 => write!(f, "{msec:.0}ms", msec = v * 1000.0),
                v => write!(f, "{sec:.1}s", sec = v),
            },
        }
    }
}
