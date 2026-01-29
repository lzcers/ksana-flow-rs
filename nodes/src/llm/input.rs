use flow::Input;

pub(crate) fn extract_input_string(input: &Input) -> String {
    input
        .get_str_as::<String>("input")
        .or_else(|| input.get_str_as::<String>("external_start"))
        .or_else(|| input.get_str_as::<String>("output"))
        .or_else(|| input.get_any_as::<String>())
        .unwrap_or_default()
}
