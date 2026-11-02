use std::io::{self, IsTerminal};
use std::process::ExitCode;

fn main() -> ExitCode {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let interactive = stdin.is_terminal() && stdout.is_terminal();
    match mini_lisp::repl::run(stdin.lock(), stdout.lock(), interactive) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mini_lisp: {error}");
            ExitCode::FAILURE
        }
    }
}
