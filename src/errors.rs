use colored::Colorize;

use crate::Span;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O Error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Tokenizer Error")]
    Tokenizer,

    #[error("Failed to compile")]
    Compile,
}

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug)]
pub enum MultiResult<T> {
    Err(Vec<CompileError>),
    Ok(T),
}

#[derive(Debug, Clone)]
pub struct CompileError {
    pub message: String,
    pub span: Span,
}

pub fn print_errors(text: &str, lines: &[usize], errors: Vec<CompileError>, max: usize) {
    let mut count = 0;
    let mut reached_max = false;
    let total_error_count = errors.len();

    for error in errors {
        if count >= max {
            reached_max = true;
            break;
        }

        print_error(text, lines, error);
        if count != total_error_count - 1 {
            println!();
        }

        count += 1;
    }

    if reached_max {
        eprintln!("... and {} more errors.", total_error_count - count);
    }
}

pub fn print_error(text: &str, lines: &[usize], error: CompileError) {
    let CompileError { message, span } = error;

    assert!((span.line.end - span.line.start) == 1);

    let line_before = (span.line.start > 0).then(|| span.line.start - 1);
    let line = span.line.start;
    let line_after = (span.line.end < lines.len()).then(|| span.line.end);

    eprintln!("{}: {}", "ERROR".red(), message);

    // Print line above the error, if possible
    if let Some(line_before) = line_before {
        eprint!("   {} ", "|".blue());
        eprint_line(text, line_before, &lines);
    }

    // Prints the line with an error
    eprint!("{:02} {} ", line.to_string().blue(), "|".blue());
    eprint_line(text, line, &lines);

    // Prints pointer
    eprint!("   {} ", "|".blue());
    for _ in 0..span.col.start {
        eprint!(" ");
    }
    for _ in 0..(span.col.end - span.col.start) {
        eprint!("{}", "^".red());
    }
    eprintln!();

    // Print line below the error, if possible
    if let Some(line_after) = line_after {
        eprint!("   {} ", "|".blue());
        eprint_line(text, line_after, &lines);
    }
}

fn get_line<'a>(text: &'a str, line: usize, lines: &'_ [usize]) -> &'a str {
    let start_idx = lines[line];
    let end_idx = if line + 1 < lines.len() {
        lines[line + 1] - 1
    } else if text.chars().nth(text.len() - 1).unwrap() == '\n' {
        text.len() - 1
    } else {
        text.len()
    };

    &text[start_idx..end_idx]
}

fn eprint_line(text: &str, line: usize, lines: &[usize]) {
    eprintln!("{}", get_line(text, line, lines));
}
