use super::body_excerpt;

#[test]
fn literal_windows_preserve_unicode_and_first_match() {
    for (needle, query) in [
        ("İSTANBUL", "i\u{307}stanbul"),
        ("ΟΣ", "ος"),
        ("ΣB", "σb"),
        ("🧬e\u{301}", "🧬e\u{301}"),
        ("İ", "\u{307}"),
    ] {
        let body = format!("{} {needle} {} {needle}", "x".repeat(700), "y".repeat(700));
        let excerpt = body_excerpt(&body, query).unwrap();
        assert!(excerpt.contains(needle));
        assert!(excerpt.contains('x'));
        assert!(excerpt.chars().count() <= 512);
        assert!(body.contains(excerpt.trim_matches('…')));
        assert_eq!(body_excerpt(&body, query).unwrap(), excerpt);
    }
}

#[test]
fn full_body_context_remains_authoritative() {
    let body = format!("AΣ{}B", "\u{301}".repeat(600));
    assert!(body.to_lowercase().contains("aσ"));
    let excerpt = body_excerpt(&body, "aσ").unwrap();
    assert!(excerpt.starts_with("AΣ"));
    assert!(!excerpt.to_lowercase().contains("aσ"));
    assert!(excerpt.chars().count() <= 512);
}

#[test]
fn boundaries_and_maximum_query_fit() {
    assert_eq!(body_excerpt("Short Body", "body").unwrap(), "Short Body");
    assert_eq!(body_excerpt("Short Body", "absent"), None);
    for position in [0, 1, 255, 510, 511, 512, 700] {
        let needle = "Q".repeat(256);
        let body = format!("{}{needle}{}", "a".repeat(position), "z".repeat(700));
        let excerpt = body_excerpt(&body, &needle.to_lowercase()).unwrap();
        assert!(excerpt.contains(&needle));
        assert!(excerpt.chars().count() <= 512);
    }
    let body = format!("{}END", "a".repeat(65533));
    let excerpt = body_excerpt(&body, "end").unwrap();
    assert!(excerpt.ends_with("END"));
    assert!(excerpt.starts_with('…'));
    assert_eq!(body_excerpt(&"a".repeat(512), "a").unwrap().len(), 512);
}
