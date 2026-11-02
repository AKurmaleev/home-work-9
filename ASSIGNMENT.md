# Homework 9: Simple Arithmetic Lisp

## Source and goal

The homework source is **HW9, “Lab3 Simple arithmetic Lisp” (10 pages)**. Build a small arithmetic Lisp interpreter with a persistent interactive environment.

**Lecture 19, “UNSAFE” (39 pages)** is separate learning material, not a requirement to use unsafe Rust. This interpreter needs no unsafe code. See [LEARNING.md](LEARNING.md) for the distinction and lecture notes.

## Homework requirements

- Support prefix arithmetic with `+`, `-`, `*`, and `/` and nested expressions.
- Accept variadic `+` and `*`; require exactly two operands for `-` and `/`.
- Support `(define name expression)` and reuse defined variables on subsequent REPL lines.
- Provide a read–evaluate–print loop with `quit` and `exit` commands.
- Report malformed input and evaluation failures as readable text rather than panicking; allow further input after an expression error.
- Include tests for arithmetic, nesting, variable persistence, and error handling.

### Required examples

These are expected numeric outcomes, not a specification of prompts or exact diagnostic wording.

| Input | Expected outcome |
| --- | --- |
| `(+ 1 2)` | `3` |
| `(* (+ 2 3) 4)` | `20` |
| `(define x 10)`, then `(+ x 5)` on the next line | `15` for the second expression |
| `(/ 20 4)` | `5` |
| `(/ 10 0)` | Division-by-zero error |
| `(+ missing 1)` | Unknown-variable error |
| `(+ 1 2` | Missing-closing-parenthesis error |

## Implementation baseline

- Package: `mini_lisp`; Rust edition `2024`; minimum Rust version `1.85`.
- Use safe Rust and the standard library only.
- Represent syntax with `Expr` variants `Number(f64)`, `Symbol(String)`, and `List(Vec<Expr>)`.
- Store numeric bindings in an `Env` backed by `HashMap<String, f64>`.
- Separate lexical tokenization, parsing, evaluation, and REPL input/output.
- Interpret the AST directly; do not compile or execute input as Rust, shell commands, or another host language.
- Exclude user-defined functions, macros, and language-level recursion. Recursive traversal of nested syntax is an implementation technique, not a Lisp function feature.

## Project design choices, not extra homework requirements

- Accept one complete expression per line; reject trailing expressions instead of silently ignoring them. No multiline expression assembly.
- Define the empty arithmetic identities: `(+)` is `0`, and `(*)` is `1`.
- Accept only finite `f64` numeric values and arithmetic results; reject NaN and infinities. Reject division by both positive and negative zero.
- Allow variable redefinition.
- Require a symbol on the left-hand side of `define`. Evaluate its right-hand side before inserting or replacing that binding; a failed right-hand side does not replace the target binding by itself.
- Evaluate operands left to right. Nested definitions are nontransactional: successful earlier mutations remain if a later evaluation fails, including mutations made while evaluating a failing right-hand side.

### Resource budgets

These are project safeguards, not PDF requirements:

- **16,384 UTF-8 payload bytes per input line**, excluding LF or CRLF. A bare CR at EOF is payload. Oversized lines are drained without evaluation; invalid UTF-8 is rejected.
- **64 nested lists**, with atoms at depth zero. Parser and evaluator use the same limit.
- **1,024 variable names created by evaluation**. Existing names can still be reassigned at the limit; library callers own and may directly populate their `Env`.
- Direct parser calls also cap token count and aggregate token bytes at 16,384. Standalone `tokenize` allocates proportionally to its caller-supplied input; `run_line` checks input size before calling it.

These limits bound retained input/state, not read duration or allocator failure. This blocking CLI is not a hostile-code sandbox.

## Acceptance checklist

- Test all four operators, their arities, nesting, and the selected empty identities.
- Test definition, lookup across lines, redefinition, invalid definition targets, and failed right-hand sides.
- Test left-to-right side effects and retention of earlier nested definitions after later errors.
- Test missing or extra parentheses, trailing expressions, unknown names, zero divisors, and non-finite values/results.
- Test recovery after expression errors, both exit commands, and end-of-input handling.
- Test resource budgets at their exact boundaries and rejection paths.

The assignment's named diagnostic cases are preserved: `missing ')'`, `unknown variable: unknown`, `division by zero`, and `'-' expects exactly 2 arguments`. The REPL prefixes diagnostics with `error: `.

See [REPORT.md](REPORT.md) for completed verification.
