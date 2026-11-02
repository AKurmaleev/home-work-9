use std::io::{self, ErrorKind, Write};

use mini_lisp::{MAX_INPUT_BYTES, repl};

fn transcript(input: &[u8]) -> io::Result<String> {
    let mut output = Vec::new();
    repl::run(input, &mut output, false)?;
    Ok(String::from_utf8(output).expect("REPL emits UTF-8"))
}

#[test]
fn session_retains_variables_and_recovers_after_language_errors() -> io::Result<()> {
    let output =
        transcript(b"(define x 10)\n(+ x 5)\n(/ x 0)\n(+ unknown 1)\n(+ 1 2\n(* x 2)\nexit\n99\n")?;
    assert_eq!(
        output,
        "10\n15\nerror: division by zero\nerror: unknown variable: unknown\nerror: missing ')'\n20\n"
    );
    Ok(())
}

#[test]
fn quit_and_exit_are_trimmed_exact_commands() -> io::Result<()> {
    for command in ["quit", "exit", "  quit  ", "\texit\r"] {
        assert_eq!(transcript(format!("{command}\n42\n").as_bytes())?, "");
    }
    assert_eq!(
        transcript(b"QUIT\n42")?,
        "error: unknown variable: QUIT\n42\n"
    );
    Ok(())
}

#[test]
fn blank_input_eof_crlf_and_unterminated_last_expression() -> io::Result<()> {
    assert_eq!(transcript(b"")?, "");
    assert_eq!(transcript(b"\n \t\r\n")?, "");
    assert_eq!(transcript(b"1\r\n(+ 2 3)")?, "1\n5\n");
    Ok(())
}

#[test]
fn bare_carriage_return_at_eof_counts_toward_payload_limit() -> io::Result<()> {
    let input = format!("1{}\r", " ".repeat(MAX_INPUT_BYTES - 1));
    assert_eq!(input.len(), MAX_INPUT_BYTES + 1);
    assert_eq!(
        transcript(input.as_bytes())?,
        "error: input exceeds byte limit\n"
    );
    let exact = format!("1{}\r", " ".repeat(MAX_INPUT_BYTES - 2));
    assert_eq!(transcript(exact.as_bytes())?, "1\n");
    Ok(())
}

#[test]
fn interactive_prompts_are_flushed_and_pipeline_mode_has_none() -> io::Result<()> {
    let mut output = Vec::new();
    repl::run(&b"42\nexit\n"[..], &mut output, true)?;
    assert_eq!(output, b"mini_lisp> 42\nmini_lisp> ");
    assert_eq!(transcript(b"42\nexit\n")?, "42\n");
    Ok(())
}

#[test]
fn transport_rejections_do_not_evaluate_input_or_poison_session() -> io::Result<()> {
    let mut input = b"(define x 7)\n(define x 999) ".to_vec();
    input.extend(vec![b' '; MAX_INPUT_BYTES]);
    input.extend_from_slice(b"\n\xff\n(+ x 1)\n");
    assert_eq!(
        transcript(&input)?,
        "7\nerror: input exceeds byte limit\nerror: input must be valid UTF-8\n8\n"
    );
    Ok(())
}

struct FailingWriter {
    fail_write: bool,
    fail_flush: bool,
}

impl Write for FailingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.fail_write {
            Err(io::Error::from(ErrorKind::BrokenPipe))
        } else {
            Ok(bytes.len())
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.fail_flush {
            Err(io::Error::from(ErrorKind::PermissionDenied))
        } else {
            Ok(())
        }
    }
}

#[test]
fn output_errors_are_returned() {
    let output = FailingWriter {
        fail_write: true,
        fail_flush: false,
    };
    assert_eq!(
        repl::run(&b"42\n"[..], output, false).unwrap_err().kind(),
        ErrorKind::BrokenPipe
    );
}

#[test]
fn flush_failures_are_returned_for_results_prompts_and_exit() {
    for (input, interactive) in [
        (&b"42\n"[..], false),
        (&b""[..], true),
        (&b""[..], false),
        (&b"exit\n"[..], false),
    ] {
        let output = FailingWriter {
            fail_write: false,
            fail_flush: true,
        };
        assert_eq!(
            repl::run(input, output, interactive).unwrap_err().kind(),
            ErrorKind::PermissionDenied
        );
    }
}

#[test]
fn short_writes_are_completed_without_losing_output() -> io::Result<()> {
    struct ShortWriter(Vec<u8>);
    impl Write for ShortWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if bytes.is_empty() {
                return Ok(0);
            }
            self.0.push(bytes[0]);
            Ok(1)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut output = ShortWriter(Vec::new());
    repl::run(&b"42\n(+ 1 2)\n"[..], &mut output, false)?;
    assert_eq!(output.0, b"42\n3\n");
    Ok(())
}
