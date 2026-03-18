use owo_colors::OwoColorize;
use std::io::IsTerminal;

fn stderr_color() -> bool {
    std::io::stderr().is_terminal()
}

pub fn header(input: &str, mode: &str, level: &str) {
    if stderr_color() {
        eprintln!(
            "{} {} {} {} {} {}",
            "distill".bold(),
            "|".dimmed(),
            input.cyan(),
            "|".dimmed(),
            mode.yellow(),
            level.dimmed(),
        );
    } else {
        eprintln!("distill | {input} | {mode} {level}");
    }
}

pub fn ingesting(source: &str) {
    if stderr_color() {
        eprintln!("{} {}", "  Ingesting".green().bold(), source.dimmed());
    } else {
        eprintln!("  Ingesting {source}");
    }
}

pub fn segmented(chunks: usize, tokens: usize) {
    if stderr_color() {
        eprintln!(
            "{} {} chunks, ~{} tokens",
            "  Segmented".green().bold(),
            chunks.to_string().white().bold(),
            tokens.to_string().white().bold(),
        );
    } else {
        eprintln!("  Segmented {chunks} chunks, ~{tokens} tokens");
    }
}

pub fn warning(msg: &str) {
    if stderr_color() {
        eprintln!("{}{} {msg}", "warn".yellow().bold(), ":".dimmed());
    } else {
        eprintln!("warn: {msg}");
    }
}

pub fn done(chunks: usize, input_tokens: usize, output_tokens: usize, output: &str) {
    let ratio = if input_tokens > 0 {
        (output_tokens as f64 / input_tokens as f64 * 100.0) as usize
    } else {
        100
    };

    if stderr_color() {
        eprintln!();
        eprintln!(
            "{} {} chunks | {} {} {} tokens (~{}%)",
            "  Done".green().bold(),
            chunks,
            input_tokens,
            "->".dimmed(),
            output_tokens.to_string().white().bold(),
            ratio,
        );
        eprintln!("    {} {}", "->".dimmed(), output.cyan());
    } else {
        eprintln!();
        eprintln!(
            "  Done {chunks} chunks | {input_tokens} -> {output_tokens} tokens (~{ratio}%)"
        );
        eprintln!("    -> {output}");
    }
}

pub fn cleaned(input: &str) {
    if stderr_color() {
        eprintln!("{} cache for {}", "Cleaned".green().bold(), input.dimmed());
    } else {
        eprintln!("Cleaned cache for {input}");
    }
}

pub fn print_error(err: &dyn std::fmt::Display) {
    if stderr_color() {
        eprintln!("{}{} {err}", "error".red().bold(), ":".dimmed());
    } else {
        eprintln!("error: {err}");
    }
}
