//! djs-cli's integration tests in one binary: every test binary links the whole
//! engine, and macOS checks each new binary before its first run (0197, D2791).

mod repl;
mod run;
