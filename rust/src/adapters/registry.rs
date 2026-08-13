use super::Adapter;
use super::test_v0_9_to_v1_0::{V0_9ToV1_0, FROM_VERSION, TO_VERSION};

/// Returns the ordered adapter chain needed to migrate from `from` to `to`.
/// Register newly generated adapters here alongside their version pair.
pub fn get_chain(from: &str, to: &str) -> Vec<Box<dyn Adapter>> {
    if from == to {
        return vec![];
    }
    if from == FROM_VERSION && to == TO_VERSION {
        return vec![Box::new(V0_9ToV1_0)];
    }
    vec![]
}
