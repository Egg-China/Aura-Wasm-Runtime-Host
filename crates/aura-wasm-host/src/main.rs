fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.as_slice() != ["--stdio"] {
        eprintln!("protocol-error: expected --stdio");
        std::process::exit(2);
    }
}
