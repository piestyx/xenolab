pub fn generate_playable(seed: u64) -> crate::engine::world::WorldRecipe {
    crate::worldgen::generator::generate(seed)
}
