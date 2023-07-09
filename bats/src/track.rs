/// Contains  information for a single track.
pub struct Track {
    /// An identifier for this track.
    pub id: u32,
    /// The plugin instance on the track.
    pub plugin_instances: Vec<PluginInstance>,
    /// If the track  should be enabled.
    pub enabled: bool,
    /// The amount to output.
    pub volume: f32,
}

/// A plugin instance.
pub struct PluginInstance {
    /// The id of the plugin instance.
    pub instance_id: u32,
    /// The id of the plugin.
    pub plugin_id: u32,
    /// The plugin instance.
    pub instance: livi::Instance,
}
