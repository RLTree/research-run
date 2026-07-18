#![forbid(unsafe_code)]

fn main() {
    if let Err(error) = research_run::cli::run() {
        eprintln!(
            "error: {}",
            research_run::cli::terminal_text(&error.to_string())
        );
        std::process::exit(2);
    }
}
