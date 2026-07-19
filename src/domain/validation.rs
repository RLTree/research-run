use std::path::{Component, Path};

use crate::{Error, Result};

use super::{FORMAT_VERSION, MAX_LIST_ITEMS, MAX_TEXT_BYTES};

pub(super) fn validate_header(version: u32, kind: &str, expected_kind: &str) -> Result<()> {
    if version != FORMAT_VERSION {
        return Err(Error::invalid(
            "schema_version",
            format!("expected {FORMAT_VERSION}, found {version}"),
        ));
    }
    if kind != expected_kind {
        return Err(Error::invalid(
            "record kind",
            format!("expected {expected_kind}"),
        ));
    }
    Ok(())
}

pub(super) fn validate_record_header(
    version: u32,
    kind: &str,
    expected: &str,
    id: &str,
) -> Result<()> {
    validate_header(version, kind, expected)?;
    validate_id(id, "id")
}

pub fn validate_id(value: &str, field: &str) -> Result<()> {
    if value.is_empty() || value.len() > 64 {
        return Err(Error::invalid(field, "must contain 1 to 64 bytes"));
    }
    let mut bytes = value.bytes();
    if !bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        || !bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(Error::invalid(
            field,
            "must start with a lowercase ASCII letter and contain only lowercase letters, digits, or '-'",
        ));
    }
    Ok(())
}

pub(super) fn validate_hex_digest(value: &str, field: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(Error::invalid(field, "must be a lowercase SHA-256 digest"));
    }
    Ok(())
}

pub fn required_text(value: &str, field: &str) -> Result<String> {
    bounded_text(value, field)?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(Error::invalid(field, "must not be empty"));
    }
    Ok(trimmed.to_owned())
}

pub(super) fn bounded_text(value: &str, field: &str) -> Result<()> {
    if value.len() > MAX_TEXT_BYTES {
        return Err(Error::Budget(format!(
            "{field} exceeds the {MAX_TEXT_BYTES} byte text budget"
        )));
    }
    Ok(())
}

pub(super) fn validate_text_list(values: &[String], field: &str, required: bool) -> Result<()> {
    if required && values.is_empty() {
        return Err(Error::invalid(field, "must contain at least one item"));
    }
    if values.len() > MAX_LIST_ITEMS {
        return Err(Error::Budget(format!(
            "{field} exceeds the {MAX_LIST_ITEMS} item budget"
        )));
    }
    for value in values {
        required_text(value, field)?;
    }
    Ok(())
}

pub fn validate_workspace_locator(value: &str) -> Result<()> {
    bounded_text(value, "workspace locator")?;
    let path = Path::new(value);
    if path.is_absolute() || value.is_empty() {
        return Err(Error::invalid(
            "workspace locator",
            "must be a non-empty relative path",
        ));
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(Error::invalid(
            "workspace locator",
            "parent traversal and absolute paths are forbidden",
        ));
    }
    if value
        .split('/')
        .any(|component| component.is_empty() || component == ".")
    {
        return Err(Error::invalid(
            "workspace locator",
            "must use canonical non-empty path components",
        ));
    }
    Ok(())
}

pub(crate) fn validate_timestamp(value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    let separators = [
        (4, b'-'),
        (7, b'-'),
        (10, b'T'),
        (13, b':'),
        (16, b':'),
        (19, b'Z'),
    ];
    if bytes.len() != 20
        || separators
            .iter()
            .any(|(index, expected)| bytes[*index] != *expected)
        || bytes.iter().enumerate().any(|(index, byte)| {
            !separators.iter().any(|(position, _)| *position == index) && !byte.is_ascii_digit()
        })
    {
        return Err(Error::invalid(
            "observed_at",
            "must use UTC YYYY-MM-DDTHH:MM:SSZ form",
        ));
    }
    for (start, end, maximum) in [(5, 7, 12), (11, 13, 23), (14, 16, 59), (17, 19, 59)] {
        let component = value[start..end]
            .parse::<u32>()
            .expect("timestamp shape proves ASCII digits");
        let minimum = u32::from(start == 5);
        if component < minimum || component > maximum {
            return Err(Error::invalid(
                "observed_at",
                "contains an out-of-range component",
            ));
        }
    }
    let year = value[0..4]
        .parse::<u32>()
        .expect("timestamp shape proves ASCII digits");
    let month = value[5..7]
        .parse::<u32>()
        .expect("timestamp shape proves ASCII digits");
    let day = value[8..10]
        .parse::<u32>()
        .expect("timestamp shape proves ASCII digits");
    let leap_year =
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let maximum_day = match month {
        2 if leap_year => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if day == 0 || day > maximum_day {
        return Err(Error::invalid(
            "observed_at",
            "contains an impossible calendar date",
        ));
    }
    Ok(())
}

pub(super) fn slug(name: &str) -> String {
    let mut result = String::new();
    let mut separator = false;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            if separator && !result.is_empty() {
                result.push('-');
            }
            result.push(character.to_ascii_lowercase());
            separator = false;
        } else {
            separator = true;
        }
    }
    if result.is_empty() {
        result.push_str("project");
    } else if !result.as_bytes()[0].is_ascii_lowercase() {
        result.insert_str(0, "project-");
    }
    result.truncate(64);
    if result.ends_with('-') {
        result.pop();
    }
    result
}
