use std::env;
use std::process::ExitCode;

use gitee_cli::run;

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(outcome) => {
            if let Some(body) = outcome.stdout {
                println!("{body}");
            }
            ExitCode::from(outcome.code)
        }
        Err(error) => {
            if let Some(body) = error.stdout {
                println!("{body}");
            }
            if let Some(message) = error.stderr {
                eprintln!("{message}");
            }
            ExitCode::from(error.code)
        }
    }
}
