use super::domain::validate_timestamp;

#[test]
fn timestamps_obey_gregorian_calendar_dates() {
    for invalid in [
        "2026/02-28T20:00:00Z",
        "A026-02-28T20:00:00Z",
        "2026-00-01T20:00:00Z",
        "2026-01-00T20:00:00Z",
        "2026-01-01T24:00:00Z",
        "2026-01-01T23:60:00Z",
        "2026-01-01T23:59:60Z",
        "2026-02-29T20:00:00Z",
        "2025-02-29T20:00:00Z",
        "1900-02-29T20:00:00Z",
        "2026-04-31T20:00:00Z",
        "2026-06-31T20:00:00Z",
        "2026-09-31T20:00:00Z",
        "2026-11-31T20:00:00Z",
    ] {
        assert!(validate_timestamp(invalid).is_err(), "{invalid}");
    }
    for valid in [
        "2024-02-29T20:00:00Z",
        "2000-02-29T20:00:00Z",
        "2026-12-31T23:59:59Z",
    ] {
        assert!(validate_timestamp(valid).is_ok(), "{valid}");
    }
}
