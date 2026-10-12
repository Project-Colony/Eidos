//! The `eidos` binary: everything lives in the library, which the window's
//! binary also links so that one file can be both (see `eidos::CLI_MARKER`).

fn main() {
    eidos::run(std::env::args().skip(1).collect());
}
