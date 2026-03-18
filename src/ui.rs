use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use owo_colors::OwoColorize;
use std::io::IsTerminal;
use std::sync::OnceLock;
use std::time::Duration;

static MULTI_STATE: OnceLock<(MultiProgress, bool, bool)> = OnceLock::new();

const TICKS: [&str; 11] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", " "];

pub struct Console {
    multi: MultiProgress,
    color: bool,
    quiet: bool,
}

impl Console {
    pub fn new(quiet: bool) -> Self {
        let color = std::io::stderr().is_terminal();
        let multi = MultiProgress::with_draw_target(ProgressDrawTarget::stderr());
        MULTI_STATE.set((multi.clone(), color, quiet)).ok();
        Self { multi, color, quiet }
    }

    pub fn spinner(&self, msg: &str) -> Spinner {
        if self.quiet {
            return Spinner {
                bar: None,
                multi: self.multi.clone(),
                color: self.color,
            };
        }
        let bar = self.multi.add(ProgressBar::new_spinner());
        bar.set_style(
            ProgressStyle::with_template("  {spinner:.dim} {msg}")
                .unwrap()
                .tick_strings(&TICKS),
        );
        bar.set_message(msg.to_string());
        bar.enable_steady_tick(Duration::from_millis(80));
        Spinner {
            bar: Some(bar),
            multi: self.multi.clone(),
            color: self.color,
        }
    }

    pub fn progress(&self, total: u64, label: &str) -> Progress {
        if self.quiet {
            return Progress {
                bar: None,
                multi: self.multi.clone(),
                color: self.color,
            };
        }
        let bar = self.multi.add(ProgressBar::new(total));
        let template = format!(
            "  {{spinner:.dim}} {} {{bar:24.cyan/dim}} {{pos}}/{{len}}",
            label
        );
        bar.set_style(
            ProgressStyle::with_template(&template)
                .unwrap()
                .tick_strings(&TICKS)
                .progress_chars("━╸─"),
        );
        bar.enable_steady_tick(Duration::from_millis(80));
        Progress {
            bar: Some(bar),
            multi: self.multi.clone(),
            color: self.color,
        }
    }

    pub fn done(&self, chunks: usize, input_tokens: usize, output_tokens: usize, output: &str) {
        if self.quiet {
            return;
        }
        let ratio = if input_tokens > 0 {
            (output_tokens as f64 / input_tokens as f64 * 100.0) as usize
        } else {
            100
        };
        if self.color {
            let _ = self.multi.println(format!(
                "  {} {} chunks | {} {} {} tokens (~{}%) {} {}",
                "✓".green(),
                chunks,
                input_tokens,
                "→".dimmed(),
                output_tokens.to_string().white().bold(),
                ratio,
                "→".dimmed(),
                output.cyan(),
            ));
        } else {
            let _ = self.multi.println(format!(
                "  ✓ {chunks} chunks | {input_tokens} → {output_tokens} tokens (~{ratio}%) → {output}"
            ));
        }
    }

    pub fn cleaned(&self, input: &str) {
        if self.color {
            let _ = self.multi.println(format!(
                "  {} Cleaned cache for {}",
                "✓".green(),
                input.dimmed()
            ));
        } else {
            let _ = self.multi.println(format!("  ✓ Cleaned cache for {input}"));
        }
    }
}

pub struct Spinner {
    bar: Option<ProgressBar>,
    multi: MultiProgress,
    color: bool,
}

impl Spinner {
    pub fn done(mut self, msg: &str) {
        if let Some(bar) = self.bar.take() {
            bar.finish_and_clear();
            print_check(&self.multi, self.color, msg);
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        if let Some(bar) = self.bar.take() {
            bar.finish_and_clear();
        }
    }
}

pub struct Progress {
    bar: Option<ProgressBar>,
    multi: MultiProgress,
    color: bool,
}

impl Progress {
    pub fn inc(&self) {
        if let Some(bar) = &self.bar {
            bar.inc(1);
        }
    }

    pub fn finish(mut self, msg: &str) {
        if let Some(bar) = self.bar.take() {
            bar.finish_and_clear();
            print_check(&self.multi, self.color, msg);
        }
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        if let Some(bar) = self.bar.take() {
            bar.finish_and_clear();
        }
    }
}

fn print_check(multi: &MultiProgress, color: bool, msg: &str) {
    if color {
        let _ = multi.println(format!("  {} {msg}", "✓".green()));
    } else {
        let _ = multi.println(format!("  ✓ {msg}"));
    }
}

pub fn warning(msg: &str) {
    if let Some((multi, color, quiet)) = MULTI_STATE.get() {
        if *quiet {
            return;
        }
        if *color {
            let _ = multi.println(format!("  {} {msg}", "⚠".yellow()));
        } else {
            let _ = multi.println(format!("  ⚠ {msg}"));
        }
    }
}

pub fn print_error(err: &dyn std::fmt::Display) {
    if std::io::stderr().is_terminal() {
        eprintln!("{}{} {err}", "error".red().bold(), ":".dimmed());
    } else {
        eprintln!("error: {err}");
    }
}
