//! Diagnostic renderer — produces human-readable output with source snippets.
//!
//! Output format is inspired by `rustc`:
//! ```text
//! error[parse]: unexpected token `}`
//!   --> example.spl:12:5
//!    |
//! 12 |     fn foo() {
//!    |              ^ expected `;`
//!    |
//! ```

use crate::diagnostic::{Diagnostic, Severity};
use crate::source::SourceFile;

/// ANSI color codes for terminal output.
mod color {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const RED: &str = "\x1b[31m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const CYAN: &str = "\x1b[36m";
    pub const BOLD_RED: &str = "\x1b[1;31m";
    pub const BOLD_YELLOW: &str = "\x1b[1;33m";
    pub const BOLD_CYAN: &str = "\x1b[1;36m";
    pub const BOLD_BLUE: &str = "\x1b[1;34m";
}

fn severity_color(sev: Severity) -> &'static str {
    match sev {
        Severity::Error => color::BOLD_RED,
        Severity::Warning => color::BOLD_YELLOW,
        Severity::Note => color::BOLD_CYAN,
    }
}

fn caret_color(sev: Severity) -> &'static str {
    match sev {
        Severity::Error => color::RED,
        Severity::Warning => color::YELLOW,
        Severity::Note => color::CYAN,
    }
}

/// Render a list of diagnostics to a string, with source context if available.
///
/// If `use_color` is true, ANSI escape codes are included.
pub fn render_diagnostics(
    diagnostics: &[Diagnostic],
    source: Option<&SourceFile>,
    use_color: bool,
) -> String {
    let mut out = String::new();
    for (i, diag) in diagnostics.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        render_one(&mut out, diag, source, use_color);
    }
    out
}

/// Render a single diagnostic to a string.
pub fn render_one(
    out: &mut String,
    diag: &Diagnostic,
    source: Option<&SourceFile>,
    use_color: bool,
) {
    let sev_col = if use_color {
        severity_color(diag.severity)
    } else {
        ""
    };
    let bold = if use_color { color::BOLD } else { "" };
    let blue = if use_color { color::BOLD_BLUE } else { "" };
    let caret_col = if use_color {
        caret_color(diag.severity)
    } else {
        ""
    };
    let reset = if use_color { color::RESET } else { "" };

    // Header: "error[parse]: message"
    out.push_str(&format!(
        "{sev_col}{sev}[{stage}]{reset}: {bold}{msg}{reset}\n",
        sev = diag.severity,
        stage = diag.stage,
        msg = diag.message,
    ));

    // If we have a span and source, show the location and snippet.
    if let (Some(label), Some(sf)) = (&diag.primary_label, source) {
        let start_lc = sf.line_col(label.start);
        let end_lc = sf.line_col(if label.end > label.start {
            label.end.saturating_sub(1)
        } else {
            label.start
        });

        // "  --> file.spl:12:5"
        out.push_str(&format!(
            "  {blue}-->{reset} {}:{}:{}\n",
            sf.name, start_lc.line, start_lc.col,
        ));

        // Calculate gutter width from line numbers we'll show.
        let last_line = end_lc.line;
        let gutter_width = format!("{}", last_line).len();

        // Blank gutter line.
        out.push_str(&format!(
            "  {blue}{blank:>gutter_width$} |{reset}\n",
            blank = "",
        ));

        // Show each source line in the span.
        for line_num in start_lc.line..=last_line {
            let line_text = sf.line_text(line_num);

            // Source line.
            out.push_str(&format!(
                "  {blue}{line_num:>gutter_width$} |{reset} {}\n",
                line_text,
            ));

            // Underline/caret line.
            let (ul_start, ul_end) = if start_lc.line == last_line {
                // Single-line span.
                (start_lc.col - 1, end_lc.col)
            } else if line_num == start_lc.line {
                (start_lc.col - 1, line_text.len())
            } else if line_num == last_line {
                (0, end_lc.col)
            } else {
                (0, line_text.len())
            };

            let padding = " ".repeat(ul_start);
            let underline_len = if ul_end > ul_start {
                ul_end - ul_start
            } else {
                1
            };
            let underline = "^".repeat(underline_len);

            let label_msg = if line_num == last_line {
                label
                    .message
                    .as_deref()
                    .map(|m| format!(" {m}"))
                    .unwrap_or_default()
            } else {
                String::new()
            };

            out.push_str(&format!(
                "  {blue}{blank:>gutter_width$} |{reset} {padding}{caret_col}{underline}{label_msg}{reset}\n",
                blank = "",
            ));
        }

        // Secondary labels.
        for sec in &diag.secondary_labels {
            let sec_lc = sf.line_col(sec.start);
            let sec_line = sf.line_text(sec_lc.line);
            let sec_msg = sec
                .message
                .as_deref()
                .map(|m| format!(" {m}"))
                .unwrap_or_default();

            out.push_str(&format!(
                "  {blue}{blank:>gutter_width$} |{reset}\n",
                blank = "",
            ));
            out.push_str(&format!(
                "  {blue}{num:>gutter_width$} |{reset} {}\n",
                sec_line,
                num = sec_lc.line,
            ));
            let sec_pad = " ".repeat(sec_lc.col.saturating_sub(1));
            let sec_end_lc = sf.line_col(sec.end.saturating_sub(1).max(sec.start));
            let sec_len = if sec_end_lc.col > sec_lc.col {
                sec_end_lc.col - sec_lc.col + 1
            } else {
                1
            };
            let sec_ul = "-".repeat(sec_len);
            out.push_str(&format!(
                "  {blue}{blank:>gutter_width$} |{reset} {sec_pad}{caret_col}{sec_ul}{sec_msg}{reset}\n",
                blank = "",
            ));
        }

        // Trailing blank gutter.
        out.push_str(&format!(
            "  {blue}{blank:>gutter_width$} |{reset}\n",
            blank = "",
        ));
    } else if let Some(sf) = source {
        // No span — just show the file name.
        out.push_str(&format!("  {blue}-->{reset} {}\n", sf.name));
    }

    // Notes.
    for note in &diag.notes {
        out.push_str(&format!(
            "  {blue}={reset} {bold}note{reset}: {note}\n"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Diagnostic;
    use crate::source::SourceFile;

    #[test]
    fn render_error_with_span() {
        let src = SourceFile::new("test.spl", "fn foo() {\n    let x = 42;\n}\n");
        let diag = Diagnostic::error("parse", "unexpected token `}`")
            .with_span(27, 28);
        let out = render_diagnostics(&[diag], Some(&src), false);
        assert!(out.contains("error[parse]: unexpected token `}`"), "got:\n{out}");
        assert!(out.contains("--> test.spl:3:1"), "got:\n{out}");
        assert!(out.contains("^"), "got:\n{out}");
    }

    #[test]
    fn render_error_without_span() {
        let src = SourceFile::new("test.spl", "hello");
        let diag = Diagnostic::error("resolve", "undefined name `foo`");
        let out = render_diagnostics(&[diag], Some(&src), false);
        assert!(out.contains("error[resolve]"), "got:\n{out}");
        assert!(out.contains("--> test.spl"), "got:\n{out}");
    }

    #[test]
    fn render_warning() {
        let diag = Diagnostic::warning("lint", "unused variable `x`")
            .with_note("prefix with `_` to suppress");
        let out = render_diagnostics(&[diag], None, false);
        assert!(out.contains("warning[lint]"), "got:\n{out}");
        assert!(out.contains("note: prefix with"), "got:\n{out}");
    }

    #[test]
    fn render_with_label_message() {
        let src = SourceFile::new("test.spl", "let x = true + 1;\n");
        let diag = Diagnostic::error("typecheck", "cannot add `bool` and `int`")
            .with_label(8, 18, "mismatched types");
        let out = render_diagnostics(&[diag], Some(&src), false);
        assert!(out.contains("mismatched types"), "got:\n{out}");
    }

    #[test]
    fn render_multiple_diagnostics() {
        let diag1 = Diagnostic::error("parse", "error one");
        let diag2 = Diagnostic::warning("lint", "warning two");
        let out = render_diagnostics(&[diag1, diag2], None, false);
        assert!(out.contains("error one"), "got:\n{out}");
        assert!(out.contains("warning two"), "got:\n{out}");
    }

    #[test]
    fn render_with_secondary_label() {
        let src = SourceFile::new("test.spl", "type Foo = Int;\ntype Foo = Bool;\n");
        let diag = Diagnostic::error("resolve", "duplicate definition `Foo`")
            .with_span(16, 31)
            .with_secondary(0, 15, "first defined here");
        let out = render_diagnostics(&[diag], Some(&src), false);
        assert!(out.contains("duplicate definition"), "got:\n{out}");
        assert!(out.contains("first defined here"), "got:\n{out}");
    }

    #[test]
    fn render_no_color() {
        let diag = Diagnostic::error("parse", "test");
        let out = render_diagnostics(&[diag], None, false);
        assert!(!out.contains("\x1b["), "should have no ANSI codes");
    }

    #[test]
    fn render_with_color() {
        let diag = Diagnostic::error("parse", "test");
        let out = render_diagnostics(&[diag], None, true);
        assert!(out.contains("\x1b["), "should have ANSI codes");
    }
}
