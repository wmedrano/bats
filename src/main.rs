fn main() {
    let world = livi::World::new();
    for (idx, plugin) in world.iter_plugins().enumerate() {
        println!("{}: {}", idx, plugin.name());
    }
    let plugin = world.iter_plugins().next().unwrap();
    let features = world.build_features(livi::FeaturesBuilder::default());
    let _instance = unsafe { plugin.instantiate(features, 44100.0) }.unwrap();
}
