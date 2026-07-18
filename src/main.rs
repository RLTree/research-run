#![forbid(unsafe_code)]

fn main() {
    if let Err(error) = research_run::cli::run() {
        eprintln!("error: {error}");
        std::process::exit(2);
    }
}
