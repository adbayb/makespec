mod adapters;
mod hexagon;
mod startup;

fn main() -> std::process::ExitCode {
    startup::cli::run()
}
