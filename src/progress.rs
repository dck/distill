use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Instant;

#[allow(dead_code)]
pub struct ProgressReporter {
    multi: MultiProgress,
    pass_bars: Vec<ProgressBar>,
    start_time: Instant,
    quiet: bool,
}

#[allow(dead_code)]
impl ProgressReporter {
    pub fn new(quiet: bool, is_multi_pass: bool) -> Self {
        let multi = MultiProgress::new();
        let mut pass_bars = Vec::new();

        if !quiet {
            if is_multi_pass {
                pass_bars.push(Self::create_bar(&multi, "Compressing", 0));
                pass_bars.push(Self::create_bar(&multi, "Deduplicating", 0));
                pass_bars.push(Self::create_bar(&multi, "Refining", 0));
            } else {
                pass_bars.push(Self::create_bar(&multi, "Compressing", 0));
            }
        }

        Self {
            multi,
            pass_bars,
            start_time: Instant::now(),
            quiet,
        }
    }

    fn create_bar(multi: &MultiProgress, label: &str, total: u64) -> ProgressBar {
        let style = ProgressStyle::default_bar()
            .template(&format!("[{{pos}}/{{len}}] {label:<16} {{bar:30}} {{msg}}"))
            .unwrap_or_else(|_| ProgressStyle::default_bar());

        let bar = multi.add(ProgressBar::new(total));
        bar.set_style(style);
        if total == 0 {
            bar.set_message("waiting");
        }
        bar
    }

    pub fn set_total(&self, pass: usize, total: u64) {
        if self.quiet || pass >= self.pass_bars.len() {
            return;
        }
        self.pass_bars[pass].set_length(total);
    }

    pub fn inc(&self, pass: usize, section: &str) {
        if self.quiet || pass >= self.pass_bars.len() {
            return;
        }
        self.pass_bars[pass].set_message(section.to_string());
        self.pass_bars[pass].inc(1);
    }

    pub fn finish_pass(&self, pass: usize) {
        if self.quiet || pass >= self.pass_bars.len() {
            return;
        }
        self.pass_bars[pass].finish_with_message("done");
    }

    pub fn finish_all(
        &self,
        chunks: usize,
        input_tokens: usize,
        output_tokens: usize,
        output_path: &str,
    ) {
        if self.quiet {
            return;
        }
        for bar in &self.pass_bars {
            bar.finish_and_clear();
        }
        let elapsed = self.start_time.elapsed();
        let mins = elapsed.as_secs() / 60;
        let secs = elapsed.as_secs() % 60;
        let ratio = if input_tokens > 0 {
            (output_tokens as f64 / input_tokens as f64 * 100.0) as usize
        } else {
            100
        };
        eprintln!(
            "\nDone in {mins}m {secs:02}s | {chunks} chunks | {input_tokens} -> {output_tokens} tokens (~{ratio}%)\n-> {output_path}"
        );
    }
}
