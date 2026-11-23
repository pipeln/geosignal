fn main() {
    if let Err(error) = geosignal::server::run_cli() {
        eprintln!("[server] {error}");
        std::process::exit(1);
    }
}
