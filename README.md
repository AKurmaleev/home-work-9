# Homework 9 — Mini Lisp

A small arithmetic Lisp interpreter in safe Rust, using only the standard library. Supports nested prefix expressions and persistent numeric variables.

## Run

Requires **Rust 1.85 or newer** with a linker for your Rust target.

```sh
cargo run --locked --quiet
```

Example session (input followed by its result; terminal prompts omitted):

```text
(+ 1 2)
3
(* (+ 2 3) 4)
20
(define x 10)
10
(+ x 5)
15
(/ 10 0)
error: division by zero
exit
```

Enter one expression per line. `quit`, `exit`, or EOF ends the session. Expression errors do not terminate it. Piped input has no prompts.

## Language

| Form | Behavior |
| --- | --- |
| `42`, `3.14`, `-10` | Finite `f64` numbers |
| `x` | Look up a numeric variable |
| `(+ a b ...)`, `(* a b ...)` | Variadic addition/multiplication; empty identities are `0`/`1` |
| `(- a b)`, `(/ a b)` | Exactly two operands; zero divisors rejected |
| `(define name expression)` | Evaluate RHS, store/redefine the variable, return its value |

Operands evaluate left to right. Successful nested definitions survive a later error; evaluation is not transactional. Floating-point rounding applies. No functions, macros, or multiline forms.

Input limits: **16 KiB per line**, **64 nested lists**, **1,024 variable names**. Invalid UTF-8 and overlong lines are rejected without evaluation. See [the assignment notes](ASSIGNMENT.md#resource-budgets) for precise boundaries.

## Navigation

- [Assignment and design choices](ASSIGNMENT.md)
- [Implementation and verification report](REPORT.md)
- [Learning notes: Lecture 19 and the interpreter](LEARNING.md)
- [Library entry points and usage example](src/lib.rs)
- [Lexer, AST, recursive-descent parser](src/syntax.rs)
- [Arithmetic and environment evaluation](src/eval.rs)
- [Bounded line reader and REPL](src/repl.rs)
- [CLI entry point](src/main.rs)
- [Language tests](tests/language.rs) · [REPL tests](tests/repl.rs)

## Verify

```sh
cargo fmt --all --check
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked --release
```

Lecture 19 covers unsafe Rust; HW9 asks for an interpreter. The solution deliberately requires no unsafe code. Verification results and platform scope are in [REPORT.md](REPORT.md).
