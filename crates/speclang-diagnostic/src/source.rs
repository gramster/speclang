//! Source file tracking with line/column resolution.

/// A source file with line-offset index for efficient span→line:column mapping.
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// File name or path (for display).
    pub name: String,
    /// The full source text.
    pub source: String,
    /// Byte offsets of the start of each line (line 1 starts at offset 0).
    line_starts: Vec<usize>,
}

/// A resolved source position (1-based line and column).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineCol {
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number (byte offset within line).
    pub col: usize,
}

impl SourceFile {
    /// Create a new source file, computing the line index.
    pub fn new(name: impl Into<String>, source: impl Into<String>) -> Self {
        let name = name.into();
        let source = source.into();
        let mut line_starts = vec![0];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        SourceFile {
            name,
            source,
            line_starts,
        }
    }

    /// Number of lines in the file.
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Convert a byte offset to a 1-based line:column.
    pub fn line_col(&self, offset: usize) -> LineCol {
        let offset = offset.min(self.source.len());
        let line = match self.line_starts.binary_search(&offset) {
            Ok(exact) => exact,
            Err(insert) => insert - 1,
        };
        let col = offset - self.line_starts[line];
        LineCol {
            line: line + 1,
            col: col + 1,
        }
    }

    /// Get the source text for a 1-based line number.
    pub fn line_text(&self, line: usize) -> &str {
        if line == 0 || line > self.line_starts.len() {
            return "";
        }
        let start = self.line_starts[line - 1];
        let end = if line < self.line_starts.len() {
            self.line_starts[line]
        } else {
            self.source.len()
        };
        self.source[start..end].trim_end_matches('\n').trim_end_matches('\r')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_first_char() {
        let sf = SourceFile::new("test.spl", "hello\nworld\n");
        assert_eq!(sf.line_col(0), LineCol { line: 1, col: 1 });
    }

    #[test]
    fn line_col_second_line() {
        let sf = SourceFile::new("test.spl", "hello\nworld\n");
        assert_eq!(sf.line_col(6), LineCol { line: 2, col: 1 });
        assert_eq!(sf.line_col(9), LineCol { line: 2, col: 4 });
    }

    #[test]
    fn line_col_end_of_file() {
        let sf = SourceFile::new("test.spl", "ab\ncd");
        assert_eq!(sf.line_col(5), LineCol { line: 2, col: 3 });
    }

    #[test]
    fn line_text_basic() {
        let sf = SourceFile::new("test.spl", "line one\nline two\nline three");
        assert_eq!(sf.line_text(1), "line one");
        assert_eq!(sf.line_text(2), "line two");
        assert_eq!(sf.line_text(3), "line three");
    }

    #[test]
    fn line_text_out_of_range() {
        let sf = SourceFile::new("test.spl", "hello");
        assert_eq!(sf.line_text(0), "");
        assert_eq!(sf.line_text(99), "");
    }

    #[test]
    fn line_count() {
        let sf = SourceFile::new("test.spl", "a\nb\nc\n");
        assert_eq!(sf.line_count(), 4); // 3 newlines → 4 line starts
    }

    #[test]
    fn empty_source() {
        let sf = SourceFile::new("empty.spl", "");
        assert_eq!(sf.line_count(), 1);
        assert_eq!(sf.line_col(0), LineCol { line: 1, col: 1 });
        assert_eq!(sf.line_text(1), "");
    }
}
