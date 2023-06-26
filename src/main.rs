fn main() {
    let world = livi::World::new();
    for (idx, plugin) in world.iter_plugins().enumerate() {
        println!("{}: {}", idx, plugin.name());
    }
}
