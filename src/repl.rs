use std::io::{self, BufRead, Write};

use crate::{Env, MAX_INPUT_BYTES, run_line};

/// Evaluate one expression per line, retaining definitions until EOF or exit.
///
/// Blank lines are ignored; trimmed `quit` or `exit` ends the session. Results
/// and recoverable `error: ...` diagnostics go to `output`. When `interactive`
/// is true, a flushed prompt precedes each read. The CLI enables it only for
/// terminal stdin and stdout, keeping piped output prompt-free.
///
/// Input accepts UTF-8, LF/CRLF, and a final unterminated line. At most
/// [`MAX_INPUT_BYTES`] payload bytes are accepted per line, excluding its line
/// terminator. Oversized or invalid UTF-8 lines are drained and rejected without
/// evaluating any part of them; subsequent lines can still run.
///
/// # Errors
///
/// Returns the first input, output, or flush error. Language/input rejections
/// are printed and do not end the session. Stream I/O is blocking; byte limits
/// bound retained memory, not the time needed to drain an unbounded stream.
///
/// ```
/// use mini_lisp::repl;
///
/// let mut output = Vec::new();
/// repl::run(&b"(define x 10)\n(+ x 5)\nexit\n"[..], &mut output, false)?;
/// assert_eq!(output, b"10\n15\n");
/// # Ok::<(), std::io::Error>(())
/// ```
pub fn run(mut input: impl BufRead, mut output: impl Write, interactive: bool) -> io::Result<()> {
    let mut env = Env::new();
    loop {
        if interactive {
            write!(output, "mini_lisp> ")?;
            output.flush()?;
        }
        match read_line(&mut input)? {
            InputLine::Eof => break,
            InputLine::Rejected(message) => writeln!(output, "error: {message}")?,
            InputLine::Accepted(line) => {
                let trimmed = line.trim();
                if matches!(trimmed, "quit" | "exit") {
                    break;
                }
                if trimmed.is_empty() {
                    continue;
                }
                match run_line(&line, &mut env) {
                    Ok(value) => writeln!(output, "{value}")?,
                    Err(message) => writeln!(output, "error: {message}")?,
                }
            }
        }
        output.flush()?;
    }
    output.flush()
}

#[derive(Debug, PartialEq)]
enum InputLine {
    Accepted(String),
    Rejected(&'static str),
    Eof,
}

fn read_line(input: &mut impl BufRead) -> io::Result<InputLine> {
    // The extra byte permits CR in an exact-limit CRLF line.
    let capacity = MAX_INPUT_BYTES + 1;
    let mut bytes = Vec::new();
    let mut oversized = false;
    let mut ended_with_newline = false;
    loop {
        let available = match input.fill_buf() {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            other => other?,
        };
        if available.is_empty() {
            if bytes.is_empty() && !oversized {
                return Ok(InputLine::Eof);
            }
            break;
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let length = newline.unwrap_or(available.len());
        if !oversized {
            if length > capacity - bytes.len() {
                oversized = true;
                bytes.clear();
            } else {
                bytes.extend_from_slice(&available[..length]);
            }
        }
        input.consume(length + usize::from(newline.is_some()));
        if newline.is_some() {
            ended_with_newline = true;
            break;
        }
    }
    if ended_with_newline && bytes.last() == Some(&b'\r') {
        bytes.truncate(bytes.len() - 1);
    }
    if oversized || bytes.len() > MAX_INPUT_BYTES {
        return Ok(InputLine::Rejected("input exceeds byte limit"));
    }
    Ok(match String::from_utf8(bytes) {
        Ok(line) => InputLine::Accepted(line),
        Err(_) => InputLine::Rejected("input must be valid UTF-8"),
    })
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, Cursor, ErrorKind, Read};

    use super::*;

    #[test]
    fn reader_distinguishes_lines_and_eof() -> io::Result<()> {
        let mut input = &b"\n42\r\nlast"[..];
        assert_eq!(read_line(&mut input)?, InputLine::Accepted(String::new()));
        assert_eq!(read_line(&mut input)?, InputLine::Accepted("42".into()));
        assert_eq!(read_line(&mut input)?, InputLine::Accepted("last".into()));
        assert_eq!(read_line(&mut input)?, InputLine::Eof);
        Ok(())
    }

    #[test]
    fn reader_accepts_exact_byte_limit_with_each_terminator() -> io::Result<()> {
        for ending in ["", "\n", "\r\n"] {
            let content = "é".repeat(MAX_INPUT_BYTES / 2);
            let input = format!("{content}{ending}");
            let mut reader = BufReader::with_capacity(7, Cursor::new(input));
            assert_eq!(read_line(&mut reader)?, InputLine::Accepted(content));
            assert_eq!(read_line(&mut reader)?, InputLine::Eof);
        }
        Ok(())
    }

    #[test]
    fn reader_rejects_oversized_line_and_resynchronizes() -> io::Result<()> {
        for extra in [1, 2, 100_000] {
            let input = format!("{}\r\n42\n", "x".repeat(MAX_INPUT_BYTES + extra));
            let mut reader = BufReader::with_capacity(7, Cursor::new(input));
            assert_eq!(
                read_line(&mut reader)?,
                InputLine::Rejected("input exceeds byte limit")
            );
            assert_eq!(read_line(&mut reader)?, InputLine::Accepted("42".into()));
            assert_eq!(read_line(&mut reader)?, InputLine::Eof);
        }
        Ok(())
    }

    #[test]
    fn reader_rejects_oversized_unterminated_input() -> io::Result<()> {
        let input = vec![b'x'; MAX_INPUT_BYTES + 2];
        let mut reader = BufReader::with_capacity(3, Cursor::new(input));
        assert_eq!(
            read_line(&mut reader)?,
            InputLine::Rejected("input exceeds byte limit")
        );
        assert_eq!(read_line(&mut reader)?, InputLine::Eof);
        Ok(())
    }

    #[test]
    fn reader_rejects_invalid_utf8_but_preserves_next_line() -> io::Result<()> {
        let mut reader = BufReader::with_capacity(1, &b"\xff\n42"[..]);
        assert_eq!(
            read_line(&mut reader)?,
            InputLine::Rejected("input must be valid UTF-8")
        );
        assert_eq!(read_line(&mut reader)?, InputLine::Accepted("42".into()));
        Ok(())
    }

    struct InterruptedOnce(bool);

    impl Read for InterruptedOnce {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            if self.0 {
                Ok(0)
            } else {
                self.0 = true;
                Err(io::Error::from(ErrorKind::Interrupted))
            }
        }
    }

    #[test]
    fn reader_retries_interrupted_reads() -> io::Result<()> {
        assert_eq!(
            read_line(&mut BufReader::new(InterruptedOnce(false)))?,
            InputLine::Eof
        );
        Ok(())
    }

    struct FailsAfterPrefix(Cursor<Vec<u8>>);

    impl Read for FailsAfterPrefix {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let count = self.0.read(buffer)?;
            if count == 0 {
                Err(io::Error::from(ErrorKind::PermissionDenied))
            } else {
                Ok(count)
            }
        }
    }

    #[test]
    fn reader_propagates_io_error_instead_of_accepting_partial_line() {
        for length in [0, 3, MAX_INPUT_BYTES + 2] {
            let source = FailsAfterPrefix(Cursor::new(vec![b'x'; length]));
            let error = read_line(&mut BufReader::with_capacity(7, source)).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::PermissionDenied);
        }
    }
}
