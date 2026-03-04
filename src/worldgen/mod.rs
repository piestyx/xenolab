pub mod acceptance;
pub mod generator;
pub mod spec;

pub fn generate_playable(seed: u64) -> crate::engine::world::WorldRecipe {
    acceptance::generate_playable(seed)
}

pub fn generate(seed: u64) -> crate::engine::world::WorldRecipe {
    generator::generate(seed)
}

pub fn generate_with_attempt(seed: u64, attempt: u32) -> crate::engine::world::WorldRecipe {
    generator::generate_with_attempt(seed, attempt)
}
