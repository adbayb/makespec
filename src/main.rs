mod adapters;
mod core;
mod startup;

fn main() {
    if let Err(error) = startup::cli::run() {
        eprintln!("makespec: {}", sanitize_terminal_text(&error.to_string()));
        std::process::exit(1);
    }
}

fn sanitize_terminal_text(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() && character != '\n' {
                '\u{fffd}'
            } else {
                character
            }
        })
        .collect()
}
