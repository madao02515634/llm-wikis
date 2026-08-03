use std::io::IsTerminal;
use std::time::Duration;

#[cfg(test)]
use indicatif::ProgressDrawTarget;
use indicatif::{ProgressBar, ProgressStyle};

pub trait QueryProgress {
    fn start(&mut self, wiki: &str);
    fn finish(&mut self);
}

pub struct TerminalQueryProgress {
    enabled: bool,
    bar: Option<ProgressBar>,
    #[cfg(test)]
    draw_target: Option<ProgressDrawTarget>,
}

impl TerminalQueryProgress {
    pub fn from_stdio() -> Self {
        Self {
            enabled: should_render_progress(
                std::io::stdout().is_terminal(),
                std::io::stderr().is_terminal(),
            ),
            bar: None,
            #[cfg(test)]
            draw_target: None,
        }
    }

    #[cfg(test)]
    fn new(
        stdout_is_terminal: bool,
        stderr_is_terminal: bool,
        draw_target: ProgressDrawTarget,
    ) -> Self {
        Self {
            enabled: should_render_progress(stdout_is_terminal, stderr_is_terminal),
            bar: None,
            draw_target: Some(draw_target),
        }
    }
}

impl QueryProgress for TerminalQueryProgress {
    fn start(&mut self, wiki: &str) {
        if !self.enabled {
            return;
        }

        self.finish();

        #[cfg(test)]
        let bar = self
            .draw_target
            .take()
            .map(|target| ProgressBar::with_draw_target(None, target))
            .unwrap_or_else(ProgressBar::new_spinner);
        #[cfg(not(test))]
        let bar = ProgressBar::new_spinner();

        let style = ProgressStyle::with_template("{spinner} {msg}")
            .expect("the static query progress template must be valid");
        bar.set_style(style);
        let wiki = sanitize_wiki_for_terminal(wiki);
        bar.set_message(format!("Querying wiki '{wiki}'..."));
        bar.enable_steady_tick(Duration::from_millis(100));
        self.bar = Some(bar);
    }

    fn finish(&mut self) {
        if let Some(bar) = self.bar.take() {
            bar.finish_and_clear();
        }
    }
}

impl Drop for TerminalQueryProgress {
    fn drop(&mut self) {
        self.finish();
    }
}

fn should_render_progress(stdout_is_terminal: bool, stderr_is_terminal: bool) -> bool {
    stdout_is_terminal && stderr_is_terminal
}

fn sanitize_wiki_for_terminal(wiki: &str) -> String {
    wiki.chars()
        .map(|character| {
            if character.is_control() {
                '�'
            } else {
                character
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use indicatif::{InMemoryTerm, ProgressDrawTarget};

    use super::{QueryProgress, TerminalQueryProgress, should_render_progress};

    fn in_memory_target() -> (InMemoryTerm, ProgressDrawTarget) {
        let terminal = InMemoryTerm::new(10, 80);
        let target = ProgressDrawTarget::term_like(Box::new(terminal.clone()));
        (terminal, target)
    }

    #[test]
    fn progress_requires_both_streams_to_be_terminals() {
        assert!(!should_render_progress(false, false));
        assert!(!should_render_progress(false, true));
        assert!(!should_render_progress(true, false));
        assert!(should_render_progress(true, true));
    }

    #[test]
    fn disabled_progress_stays_inactive() {
        let (_, target) = in_memory_target();
        let mut progress = TerminalQueryProgress::new(false, true, target);

        progress.start("rust");

        assert!(progress.bar.is_none());
    }

    #[test]
    fn enabled_progress_starts_with_the_query_message() {
        let (_, target) = in_memory_target();
        let mut progress = TerminalQueryProgress::new(true, true, target);

        progress.start("Rust 語言");

        let bar = progress.bar.as_ref().expect("spinner should be active");
        assert_eq!(bar.message(), "Querying wiki 'Rust 語言'...");
    }

    #[test]
    fn progress_message_replaces_terminal_control_characters() {
        let (_, target) = in_memory_target();
        let mut progress = TerminalQueryProgress::new(true, true, target);

        progress.start("ru\x1b[2Jst\r\n維基");

        let message = progress
            .bar
            .as_ref()
            .expect("spinner should be active")
            .message();
        for control in ['\x1b', '\r', '\n'] {
            assert!(!message.contains(control));
        }
        assert_eq!(message, "Querying wiki 'ru�[2Jst��維基'...");
    }

    #[test]
    fn finish_is_idempotent_and_leaves_progress_inactive() {
        let (_, target) = in_memory_target();
        let mut progress = TerminalQueryProgress::new(true, true, target);
        progress.start("rust");

        progress.finish();
        progress.finish();

        assert!(progress.bar.is_none());
    }

    #[test]
    fn disabled_terminal_combinations_do_not_touch_the_draw_target() {
        for (stdout_is_terminal, stderr_is_terminal) in
            [(false, false), (false, true), (true, false)]
        {
            let (terminal, target) = in_memory_target();
            let mut progress =
                TerminalQueryProgress::new(stdout_is_terminal, stderr_is_terminal, target);

            progress.start("rust");
            progress.finish();

            assert_eq!(terminal.contents(), "");
            assert_eq!(terminal.moves_since_last_check(), "");
        }
    }

    #[test]
    fn drop_clears_an_active_spinner() {
        let (terminal, target) = in_memory_target();
        let mut progress = TerminalQueryProgress::new(true, true, target);
        progress.start("rust");
        progress
            .bar
            .as_ref()
            .expect("spinner should be active")
            .force_draw();
        assert!(!terminal.contents().is_empty());

        drop(progress);

        assert_eq!(terminal.contents(), "");
    }
}
