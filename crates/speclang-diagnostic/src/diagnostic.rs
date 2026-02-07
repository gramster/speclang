//! Diagnostic types: severity, labels, and the unified `Diagnostic` struct.

use std::fmt;

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Informational note.
    Note,
    /// Warning (compilation proceeds).
    Warning,
    /// Error (compilation aborted).
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Note => write!(f, "note"),
            Severity::Warning => write!(f, "warning"),
            Severity::Error => write!(f, "error"),
        }
    }
}

/// A source label: a span within a source file with an optional annotation.
#[derive(Debug, Clone)]
pub struct Label {
    /// Byte offset start (inclusive).
    pub start: usize,
    /// Byte offset end (exclusive).
    pub end: usize,
    /// Optional message for this label.
    pub message: Option<String>,
}

impl Label {
    /// Create a label from a byte-offset span.
    pub fn span(start: usize, end: usize) -> Self {
        Label {
            start,
            end,
            message: None,
        }
    }

    /// Create a label with a message.
    pub fn span_msg(start: usize, end: usize, msg: impl Into<String>) -> Self {
        Label {
            start,
            end,
            message: Some(msg.into()),
        }
    }

    /// Label at a single point (zero-width).
    pub fn point(offset: usize) -> Self {
        Label {
            start: offset,
            end: offset,
            message: None,
        }
    }
}

/// A structured diagnostic from any compiler pipeline stage.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Severity level.
    pub severity: Severity,
    /// The primary error/warning message.
    pub message: String,
    /// The pipeline stage that produced this diagnostic (e.g., "parse", "resolve").
    pub stage: &'static str,
    /// Primary label (source span for the main error site). Optional because
    /// some semantic errors don't yet carry span information.
    pub primary_label: Option<Label>,
    /// Additional labels (e.g., "first defined here").
    pub secondary_labels: Vec<Label>,
    /// Additional notes appended after the diagnostic.
    pub notes: Vec<String>,
}

impl Diagnostic {
    /// Create a new error diagnostic.
    pub fn error(stage: &'static str, message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Error,
            message: message.into(),
            stage,
            primary_label: None,
            secondary_labels: vec![],
            notes: vec![],
        }
    }

    /// Create a new warning diagnostic.
    pub fn warning(stage: &'static str, message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            message: message.into(),
            stage,
            primary_label: None,
            secondary_labels: vec![],
            notes: vec![],
        }
    }

    /// Create a new note diagnostic.
    pub fn note(stage: &'static str, message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Note,
            message: message.into(),
            stage,
            primary_label: None,
            secondary_labels: vec![],
            notes: vec![],
        }
    }

    /// Attach a primary source span.
    pub fn with_span(mut self, start: usize, end: usize) -> Self {
        self.primary_label = Some(Label::span(start, end));
        self
    }

    /// Attach a primary span with label message.
    pub fn with_label(mut self, start: usize, end: usize, msg: impl Into<String>) -> Self {
        self.primary_label = Some(Label::span_msg(start, end, msg));
        self
    }

    /// Add a secondary label.
    pub fn with_secondary(mut self, start: usize, end: usize, msg: impl Into<String>) -> Self {
        self.secondary_labels
            .push(Label::span_msg(start, end, msg));
        self
    }

    /// Add a note.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Check if this diagnostic has any source location.
    pub fn has_span(&self) -> bool {
        self.primary_label.is_some()
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.severity, self.message)
    }
}
