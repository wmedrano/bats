/// Contains  information for a single track.
pub struct Track {
    /// The plugin instance on the track.
    pub plugin_instances: Vec<livi::Instance>,
    /// If the track  should be enabled.
    pub enabled: bool,
    /// The amount to output.
    pub volume: f32,
}
