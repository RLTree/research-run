type EvidenceCollection = Vec<String>;

union ObservationValue {
    integer: u64,
    decimal: f64,
}

macro_rules! render_observation {
    ($value:expr) => {
        format!("{}", $value)
    };
}

fn parse_observation_fixture() {
    let _raw = r###"raw delimiters { do not } alter function spans"###;
    /* Block-comment braces { also do not alter } spans. */
    let _ = render_observation!(EvidenceCollection::new().len());
}
