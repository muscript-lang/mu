fn main() {
    if let Err(err) = muc::cli::run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
