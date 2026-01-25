pub fn build_user_prompt(template: &str, input: &str) -> String {
    if input.is_empty() {
        return template.to_owned();
    }

    if template.contains("{input}") {
        return template.replace("{input}", input);
    }

    if template.trim().is_empty() {
        return input.to_owned();
    }

    format!("{}\n\n{}", template, input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_input_placeholder() {
        let out = build_user_prompt("Translate: {input}", "你好");
        assert_eq!(out, "Translate: 你好");
    }

    #[test]
    fn appends_input_when_no_placeholder() {
        let out = build_user_prompt("You are helpful.", "你好");
        assert_eq!(out, "You are helpful.\n\n你好");
    }

    #[test]
    fn uses_only_input_when_template_empty() {
        let out = build_user_prompt("", "你好");
        assert_eq!(out, "你好");
    }

    #[test]
    fn uses_only_template_when_input_empty() {
        let out = build_user_prompt("Say hello", "");
        assert_eq!(out, "Say hello");
    }
}
