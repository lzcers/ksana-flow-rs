use flow::NodeInputs;

pub(crate) fn extract_input_string(inputs: &NodeInputs) -> String {
    if let Some(s) = inputs.get::<String>("input") {
        return s.clone();
    }
    if let Some(s) = inputs.get::<String>("external_start") {
        return s.clone();
    }
    if let Some(s) = inputs.get::<String>("output") {
        return s.clone();
    }

    inputs
        .iter_unwrapped()
        .find_map(|(_, any)| any.as_any().downcast_ref::<String>().cloned())
        .unwrap_or_default()
}
