use term;

#[allow(dead_code)]
pub fn print_message(msg: &str, color: term::color::Color) {
    let term_stdout = term::stdout();

    if let Some(mut terminal) = term_stdout {
        let _ = terminal.fg(color);
        println!("{}", msg);
        let _ = terminal.reset();
    } else {
        println!("{}", msg);
    }
}

fn red(s: &str) {
    let mut term_stderr = term::stderr();
    term_stderr.as_mut().map(|t|{
        let _ = t.attr(term::Attr::Bold);
        let _ = t.fg(term::color::RED);
    });
    eprint!("{}", s);
    let _ = term_stderr.map(|mut t| t.reset());
}


pub fn report_error(e: &super::Error) {
    red("error:");
    eprint!(" {}", e);
    for e in e.iter().skip(1) {
        red("  caused by:");
        eprint!(" {}", e);
    }
}

/// The default target to pass to cargo, to workaround issue #11.
#[cfg(target_os="macos")]
pub fn default_target() -> &'static str {
    "x86_64-apple-darwin"
}

/// The default target to pass to cargo, to workaround issue #11.
#[cfg(not(target_os="macos"))]
pub fn default_target() -> &'static str {
    "x86_64-unknown-linux-gnu"
}
