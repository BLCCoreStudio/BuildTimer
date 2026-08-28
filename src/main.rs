use buildtimer::{
    clear_history, command_start_error_code, compare_duration, data_file_path, duration_to_millis,
    format_duration_ms, history_entries, record_run, run_wrapped, Trend,
};
use std::env;
use std::ffi::OsString;
use std::process;

const HELP: &str = "BuildTimer - measure command duration and compare local runs\n\n\
Usage:\n  buildtimer -- <command> [args...]\n  buildtimer history\n  buildtimer clear\n\n\
Options:\n  -h, --help       Show this help\n  -V, --version    Show the version\n";

fn main() {
    process::exit(run());
}

fn run() -> i32 {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    let Some(first) = args.first() else {
        eprintln!("{HELP}");
        return 2;
    };

    match first.to_str() {
        Some("history") if args.len() == 1 => show_history(),
        Some("clear") if args.len() == 1 => clear(),
        Some("-h" | "--help") if args.len() == 1 => {
            print!("{HELP}");
            0
        }
        Some("-V" | "--version") if args.len() == 1 => {
            println!("buildtimer {}", env!("CARGO_PKG_VERSION"));
            0
        }
        Some("--") => run_command(&args[1..]),
        Some("history" | "clear" | "-h" | "--help" | "-V" | "--version") => {
            eprintln!("buildtimer: unexpected arguments\n\n{HELP}");
            2
        }
        _ => {
            eprintln!("buildtimer: expected `--` before the wrapped command\n\n{HELP}");
            2
        }
    }
}

fn run_command(command: &[OsString]) -> i32 {
    if command.is_empty() {
        eprintln!("buildtimer: no wrapped command was provided after `--`");
        return 2;
    }

    let outcome = match run_wrapped(command) {
        Ok(outcome) => outcome,
        Err(error) => {
            eprintln!("buildtimer: failed to start command: {error}");
            return command_start_error_code(&error);
        }
    };

    let duration_ms = duration_to_millis(outcome.duration);
    println!("BuildTimer: {}", format_duration_ms(duration_ms));

    match data_file_path() {
        Ok(path) => match record_run(&path, command, outcome.duration) {
            Ok(Some(previous)) => print_comparison(previous.duration_ms, duration_ms),
            Ok(None) => println!("Previous: no earlier run for this command"),
            Err(error) => eprintln!("buildtimer: warning: could not save history: {error}"),
        },
        Err(error) => eprintln!("buildtimer: warning: history is unavailable: {error}"),
    }

    outcome.exit_code
}

fn show_history() -> i32 {
    let path = match data_file_path() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("buildtimer: could not locate history: {error}");
            return 1;
        }
    };

    let entries = match history_entries(&path) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("buildtimer: could not read history: {error}");
            return 1;
        }
    };

    if entries.is_empty() {
        println!("No BuildTimer history yet.");
        return 0;
    }

    println!("{:<12} {:>12}  COMMAND", "TIMESTAMP", "DURATION");
    for entry in entries.iter().rev() {
        let timestamp = entry.timestamp_unix;
        let duration = format_duration_ms(entry.duration_ms);
        let command = &entry.command;
        println!("{timestamp:<12} {duration:>12}  {command}");
    }

    0
}

fn clear() -> i32 {
    let path = match data_file_path() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("buildtimer: could not locate history: {error}");
            return 1;
        }
    };

    match clear_history(&path) {
        Ok(true) => {
            println!("BuildTimer history cleared.");
            0
        }
        Ok(false) => {
            println!("BuildTimer history is already empty.");
            0
        }
        Err(error) => {
            eprintln!("buildtimer: could not clear history: {error}");
            1
        }
    }
}

fn print_comparison(previous_ms: u64, current_ms: u64) {
    let previous = format_duration_ms(previous_ms);
    let current = format_duration_ms(current_ms);

    match compare_duration(previous_ms, current_ms) {
        Trend::Faster { delta_ms, percent } => {
            let delta = format_duration_ms(delta_ms);
            let suffix = percent_suffix(percent);
            println!("Previous: {previous} -> {current} ({delta} faster{suffix})");
        }
        Trend::Slower { delta_ms, percent } => {
            let delta = format_duration_ms(delta_ms);
            let suffix = percent_suffix(percent);
            println!("Previous: {previous} -> {current} ({delta} slower{suffix})");
        }
        Trend::Same => println!("Previous: {previous} -> {current} (same duration)"),
    }
}

fn percent_suffix(percent: Option<f64>) -> String {
    match percent {
        Some(percent) => format!(", {percent:.1}%"),
        None => String::new(),
    }
}
