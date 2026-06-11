pub fn redact_sensitive(input: &str) -> String {
    let mut output = input.to_string();
    for marker in [
        "access_token",
        "refresh_token",
        "id_token",
        "api_key",
        "OPENAI_API_KEY",
        "authorization",
        "code",
    ] {
        output = redact_all_after_marker(&output, marker);
    }
    output
}

fn redact_all_after_marker(input: &str, marker: &str) -> String {
    let mut output = input.to_string();
    let marker_lower = marker.to_ascii_lowercase();
    let mut search_start = 0;

    loop {
        let lower = output.to_ascii_lowercase();
        let Some(relative_index) = lower[search_start..].find(&marker_lower) else {
            break;
        };
        let index = search_start + relative_index;
        output = redact_one_after_marker(&output, marker, index);
        search_start = index + marker.len();
        if search_start >= output.len() {
            break;
        }
    }

    output
}

fn redact_one_after_marker(input: &str, marker: &str, index: usize) -> String {
    let mut chars: Vec<char> = input.chars().collect();
    let start = index + marker.len();
    let redact_through_spaces = marker.eq_ignore_ascii_case("authorization");
    let mut redacting = false;
    for ch in chars.iter_mut().skip(start) {
        if !redacting && (*ch == ':' || *ch == '=' || *ch == '"' || ch.is_whitespace()) {
            continue;
        }
        let is_boundary = *ch == '"'
            || *ch == ','
            || *ch == '&'
            || *ch == '}'
            || *ch == '\n'
            || (*ch).is_whitespace() && !redact_through_spaces;
        if is_boundary {
            if redacting {
                break;
            }
            continue;
        }
        redacting = true;
        *ch = '*';
    }
    chars.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::redact_sensitive;

    #[test]
    fn redacts_token_fields() {
        let input = r#"{"access_token":"access-fixture-secret","refresh_token":"refresh-fixture-secret","id_token":"id-fixture-secret"}"#;
        let output = redact_sensitive(input);

        assert!(!output.contains("access-fixture-secret"));
        assert!(!output.contains("refresh-fixture-secret"));
        assert!(!output.contains("id-fixture-secret"));
        assert!(output.contains("\"access_token\":\"*********************\""));
    }

    #[test]
    fn redacts_api_key_fields() {
        let input = r#"OPENAI_API_KEY=sk-fixture-secret api_key: another-fixture-secret"#;
        let output = redact_sensitive(input);

        assert!(!output.contains("sk-fixture-secret"));
        assert!(!output.contains("another-fixture-secret"));
        assert!(output.contains("OPENAI_API_KEY=*****************"));
        assert!(output.contains("api_key: **********************"));
    }

    #[test]
    fn redacts_authorization_header() {
        let input = "Authorization: Bearer fixture-bearer-token";
        let output = redact_sensitive(input);

        assert!(!output.contains("Bearer"));
        assert!(!output.contains("fixture-bearer-token"));
        assert!(output.starts_with("Authorization: "));
        assert!(output
            .trim_start_matches("Authorization: ")
            .chars()
            .all(|ch| ch == '*'));
    }

    #[test]
    fn redacts_callback_code_query_value() {
        let input =
            "http://localhost:1455/auth/callback?code=fixture-code-secret&state=fixture-state";
        let output = redact_sensitive(input);

        assert!(!output.contains("fixture-code-secret"));
        assert!(output.contains("code=*******************"));
        assert!(output.contains("state=fixture-state"));
    }
}
