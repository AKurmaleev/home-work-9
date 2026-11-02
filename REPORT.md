# Homework 9 — Implementation Report

## Result

Implemented the arithmetic Lisp assignment as `mini_lisp` 1.0.0: a standard-library-only Rust library and CLI. All seven required test scenarios are covered. No user-defined functions, macros, or unsafe code were added.

## Implementation

1. `tokenize` splits Unicode whitespace and ASCII parentheses while preserving atom text.
2. `parse` builds an owned `Expr` tree and rejects incomplete syntax or trailing expressions. Cursor helpers return the next unconsumed token index.
3. `eval` borrows the tree, computes arithmetic, and looks up or updates a caller-owned `Env`.
4. `repl::run` retains one environment across input lines. It reports recoverable errors and continues; I/O failures propagate to the CLI, which exits unsuccessfully.

`define` treats its target as a name rather than looking it up. It evaluates the RHS before updating that binding. Completed nested definitions are not rolled back on a later failure. Numeric operations use finite `f64` values with normal rounding, explicit zero-divisor checks, and rejection of non-finite intermediate results.

The reader bounds retained input, drains rejected lines, handles UTF-8 and LF/CRLF, and accepts a final unterminated expression. Parser/evaluator depth and environment limits are specified in [ASSIGNMENT.md](ASSIGNMENT.md).

## Verification

Executed on native Windows 11, **`x86_64-pc-windows-gnu`**:

- Rust **1.99.1**, Cargo **1.99.1**: formatting, all-target checks, Clippy, debug and release tests, and rustdoc all passed.
- Rust **1.85.0 GNU**: the same correctness tests and doctests passed, validating the declared minimum version on this target.
- **51 ordinary tests**: 7 direct reader tests, 35 language tests, 9 REPL integration tests.
- **2 doctests**: library usage and an in-memory REPL session.

Commands used from the repository root (`CARGO_INCREMENTAL=0` was set for local builds):

```sh
cargo fmt --all --check
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings -D unused_must_use -D clippy::let_underscore_must_use -D clippy::unused_result_ok
cargo test --locked
cargo test --locked --release
cargo +1.85.0-x86_64-pc-windows-gnu test --locked
cargo doc --locked --no-deps
```

Coverage includes the assignment examples, AST/cursor structure, malformed tokens and parentheses, arity, variable persistence/redefinition, failed RHS, left-to-right side effects, signed zero, finite-number boundaries, exact resource limits, invalid encoding, short reads/writes, interrupted reads, injected I/O/flush failures, and recovery after rejected input. Deterministically generated arithmetic cases supplement the fixed examples; no external fuzzer was used.

Independent review found a bare-CR-at-EOF length-boundary defect. An unchanged regression test first failed with `1\n` instead of the expected rejection, then passed after stripping CR only when followed by LF.

A release CLI smoke run produced:

```text
10
16
error: division by zero
20
```

for `(define x 10)`, `(+ x (* 2 3))`, `(/ 10 0)`, and `(* (+ 2 3) 4)`, followed by `exit`.

## Scope

Verification is local GNU-target evidence, not a GitHub Actions or cross-platform result. MSVC and Linux were not validated. No Miri run, performance benchmark, universal soundness proof, or sandbox guarantee is claimed. The interpreter remains a small blocking educational program; resource limits do not provide stream timeouts or recover from process-wide allocation failure.
