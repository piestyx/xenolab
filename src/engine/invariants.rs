use crate::engine::ids::NodeId;
use crate::engine::math::is_valid_state_value;
use crate::engine::world::WorldState;

pub fn assert_state_in_bounds(state: &WorldState) -> Result<(), String> {
    for node in NodeId::ALL {
        let value = state.get(node);
        if !is_valid_state_value(value) {
            return Err(format!(
                "state value out of bounds for {}: {}",
                node.stable_name(),
                value
            ));
        }
    }
    Ok(())
}

pub fn state_delta(a: &WorldState, b: &WorldState) -> Vec<(String, f32)> {
    NodeId::ALL
        .iter()
        .map(|node| {
            let delta = b.get(*node) - a.get(*node);
            (node.stable_name().to_string(), delta)
        })
        .collect()
}
