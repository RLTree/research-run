#![forbid(unsafe_code)]

fn main() {
    if let Err(error) = research_run::cli::run() {
        let _ = research_run::cli::print_error(&error);
        std::process::exit(error.exit_code());
    }
}
