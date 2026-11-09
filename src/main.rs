mod adapters;
mod core;
mod startup;

fn main() {
    if let Err(error) = startup::cli::run() {
        let safe_error: String = error
            .to_string()
            .chars()
            .map(|character| {
                if character.is_control() && character != '\n' {
                    '\u{fffd}'
                } else {
                    character
                }
            })
            .collect();

        eprintln!("makespec: {safe_error}");
        std::process::exit(1);
    }
}
