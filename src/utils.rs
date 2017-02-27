use std::io;
use std::io::Write;
use term;

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

pub fn write_to_stderr(err_msg: &str) {
    let term_stderr = term::stderr();
    
    if let Some(mut terminal) = term_stderr {
        let _ = terminal.attr(term::Attr::Bold);
        let _ = terminal.fg(term::color::RED);
        write!(io::stderr(), "Error: ")
            .expect("failed writing to stderr");
        let _ = terminal.fg(term::color::WHITE);
        writeln!(io::stderr(), "{}", err_msg)
            .expect("failed writing to stderr");
        let _ = terminal.reset();
    } else {
        writeln!(io::stderr(), "{}", err_msg)
            .expect("failed writing to stderr");
    }    
}
