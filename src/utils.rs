use term;

#[derive(Clone, Copy)]
pub struct TermOutputWrapper {
    color: bool,
}

impl TermOutputWrapper {
    pub fn new(color: bool) -> TermOutputWrapper {
        TermOutputWrapper { color }
    }

    #[allow(dead_code)]
    pub fn print_message(&self, msg: &str, color: term::color::Color) {
        let term_stdout = term::stdout();

        if self.color {
            if let Some(mut terminal) = term_stdout {
                let _ = terminal.fg(color);
                println!("{}", msg);
                let _ = terminal.reset();
            } else {
                println!("{}", msg);
            }
        } else {
            println!("{}", msg);
        }
    }

    fn red(&self, s: &str) {
        if self.color {
            let mut term_stderr = term::stderr();
            term_stderr.as_mut().map(|t| {
                let _ = t.attr(term::Attr::Bold);
                let _ = t.fg(term::color::RED);
            });
            eprint!("{}", s);
            let _ = term_stderr.map(|mut t| t.reset());
        } else {
            eprint!("{}", s);
        }
    }

    pub fn report_error(&self, e: &super::Error) {
        self.red("error:");
        eprintln!(" {}", e);
        for e in e.iter().skip(1) {
            self.red("  caused by:");
            eprintln!(" {}", e);
        }
    }
}

/// The default target to pass to cargo, to workaround issue #11.
#[cfg(target_os = "macos")]
pub fn default_target() -> &'static str {
    "x86_64-apple-darwin"
}

/// The default target to pass to cargo, to workaround issue #11.
#[cfg(not(target_os = "macos"))]
pub fn default_target() -> &'static str {
    "x86_64-unknown-linux-gnu"
}
