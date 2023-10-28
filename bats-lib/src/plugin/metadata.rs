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
    /// A percentage between 0% and 100%.
    Percent,
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

impl Param {
    /// Create a new bool parameter.
    pub const fn new_bool(id: u32, name: &'static str, default_value: bool) -> Param {
        Param {
            id,
            name,
            param_type: ParamType::Bool,
            default_value: if default_value { 0.55 } else { 0.45 },
            min_value: 0.45,
            max_value: 0.55,
        }
    }
}

impl Default for &'static Metadata {
    fn default() -> &'static Metadata {
        &Metadata {
            name: "default-plugin",
            params: &[],
        }
    }
}
