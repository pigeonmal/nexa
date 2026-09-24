fn main() {
    if let Err(message) = nexa_cli::run() {
        eprintln!("error: {message}");
        std::process::exit(1);
    }
}
