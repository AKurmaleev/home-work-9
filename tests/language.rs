use mini_lisp::{
    Env, Expr, MAX_DEPTH, MAX_INPUT_BYTES, MAX_VARIABLES, eval, parse, parse_expr, parse_list,
    run_line, tokenize,
};

#[test]
fn adds_numbers() {
    assert_eq!(run_line("(+ 1 2)", &mut Env::new()), Ok(3.0));
}

#[test]
fn multiplies_numbers() {
    assert_eq!(run_line("(* 3 4)", &mut Env::new()), Ok(12.0));
}

#[test]
fn divides_numbers() {
    assert_eq!(run_line("(/ 20 4)", &mut Env::new()), Ok(5.0));
}

#[test]
fn evaluates_nested_arithmetic() {
    assert_eq!(run_line("(* (+ 2 3) 4)", &mut Env::new()), Ok(20.0));
    assert_eq!(run_line("(* (+ 2 3) (- 10 6))", &mut Env::new()), Ok(20.0));
    assert_eq!(run_line("(+ 1 (* 2 3))", &mut Env::new()), Ok(7.0));
    assert_eq!(run_line("(* (+ 1 2) (- 10 4))", &mut Env::new()), Ok(18.0));
}

#[test]
fn definitions_persist_between_lines() {
    let mut env = Env::new();
    assert_eq!(run_line("(define x 10)", &mut env), Ok(10.0));
    assert_eq!(run_line("(+ x 5)", &mut env), Ok(15.0));
    assert_eq!(run_line("x", &mut env), Ok(10.0));
}

#[test]
fn reports_unknown_variables_exactly() {
    assert_eq!(
        run_line("unknown", &mut Env::new()).unwrap_err(),
        "unknown variable: unknown"
    );
    assert_eq!(
        run_line("(+ unknown 1)", &mut Env::new()).unwrap_err(),
        "unknown variable: unknown"
    );
    assert_eq!(
        run_line("(+ missing 1)", &mut Env::new()).unwrap_err(),
        "unknown variable: missing"
    );
}

#[test]
fn reports_zero_division_exactly() {
    assert_eq!(
        run_line("(/ 10 0)", &mut Env::new()).unwrap_err(),
        "division by zero"
    );
}

#[test]
fn reports_missing_parenthesis_exactly() {
    assert_eq!(
        run_line("(+ 1 2", &mut Env::new()).unwrap_err(),
        "missing ')'"
    );
}

#[test]
fn tokenizes_adjacent_parentheses_and_unicode_whitespace() {
    let expected = ["(", "+", "-2.5", "(", "*", "+3", "4e-2", ")", "name", ")"];
    assert_eq!(
        tokenize("\t(+ -2.5(* +3 4e-2)\u{2003}name)\r\n"),
        expected.map(str::to_owned).to_vec()
    );
    assert!(tokenize(" \t\r\n\u{2003}").is_empty());
    assert_eq!(tokenize("привет"), vec!["привет".to_owned()]);
    assert_eq!(tokenize("a;b"), vec!["a;b".to_owned()]);
}

#[test]
fn parses_number_symbol_and_nested_list_variants() {
    assert_eq!(parse(&tokenize("-2.5")), Ok(Expr::Number(-2.5)));
    assert_eq!(parse(&tokenize("x")), Ok(Expr::Symbol("x".to_owned())));
    assert_eq!(
        parse(&tokenize("(+ 1 (* x 3))")),
        Ok(Expr::List(vec![
            Expr::Symbol("+".to_owned()),
            Expr::Number(1.0),
            Expr::List(vec![
                Expr::Symbol("*".to_owned()),
                Expr::Symbol("x".to_owned()),
                Expr::Number(3.0),
            ]),
        ]))
    );
    assert_eq!(parse(&tokenize("()")), Ok(Expr::List(Vec::new())));
}

#[test]
fn parser_cursors_point_to_the_next_unconsumed_token() {
    let tokens = tokenize("7 (+ 1 (* 2 3)) tail");
    assert_eq!(parse_expr(&tokens, 0), Ok((Expr::Number(7.0), 1)));
    let (expression, next) = parse_expr(&tokens, 1).unwrap();
    assert_eq!(next, 10);
    assert_eq!(parse_list(&tokens, 2), Ok((expression, next)));
    assert_eq!(
        parse_expr(&tokens, next),
        Ok((Expr::Symbol("tail".to_owned()), tokens.len()))
    );
    assert_eq!(parse_list(&tokens, 5).unwrap().1, 9);
    assert_eq!(parse_list(&tokenize("() next"), 1).unwrap().1, 2);
    assert_eq!(parse_list(&tokenize("(+ 1 2)"), 1).unwrap().1, 5);
}

#[test]
fn invalid_cursors_are_recoverable() {
    let tokens = tokenize("(1)");
    for pos in [tokens.len(), tokens.len() + 1, usize::MAX] {
        assert!(parse_expr(&tokens, pos).is_err());
    }
    for start in [0, 2, tokens.len() + 1, usize::MAX] {
        assert!(parse_list(&tokens, start).is_err());
    }
    assert_eq!(parse_list(&tokenize("("), 1).unwrap_err(), "missing ')'");
}

#[test]
fn rejects_bad_parentheses_and_trailing_expressions() {
    for source in ["(", "(+ 1", "(+ (* 2 3)"] {
        assert_eq!(
            parse(&tokenize(source)).unwrap_err(),
            "missing ')'",
            "{source}"
        );
    }
    for source in [")", ") 1", "(+) )", "1 2", "(+ 1) (+ 2)", "1 ("] {
        assert!(parse(&tokenize(source)).is_err(), "{source}");
    }
    assert_eq!(parse(&tokenize(")")).unwrap_err(), "unexpected ')'");
    assert_eq!(
        parse(&tokenize("1 2")).unwrap_err(),
        "extra tokens after expression"
    );
    assert!(parse(&[]).is_err());
    assert!(run_line(" \t\n", &mut Env::new()).is_err());
}

#[test]
fn rejects_malformed_external_tokens() {
    for token in ["", "one two", "one\u{2003}two", "(x", "x)", "()"] {
        let tokens = [token.to_owned()];
        assert_eq!(parse(&tokens).unwrap_err(), "invalid token", "{token:?}");
        assert!(parse_expr(&tokens, 0).is_err());
        assert!(parse_list(&tokens, 0).is_err());
    }
}

#[test]
fn failed_float_parses_become_symbols_without_numeric_heuristics() {
    for source in ["+", "-", "1e", "1_000", "12abc", "--1", "NaNx"] {
        assert_eq!(
            parse(&tokenize(source)),
            Ok(Expr::Symbol(source.to_owned())),
            "{source}"
        );
    }
    for (source, value) in [(".5", 0.5), ("5.", 5.0), ("+2.5E2", 250.0), ("007", 7.0)] {
        assert_eq!(parse(&tokenize(source)), Ok(Expr::Number(value)));
    }
}

#[test]
fn supports_variadic_identity_and_single_argument_forms() {
    for (source, value) in [
        ("(+)", 0.0),
        ("(*)", 1.0),
        ("(+ 7)", 7.0),
        ("(* 7)", 7.0),
        ("(+ 1 2 3 4)", 10.0),
        ("(* 1 2 3 4)", 24.0),
        ("(- 10 4)", 6.0),
        ("(/ 9 2)", 4.5),
    ] {
        assert_eq!(run_line(source, &mut Env::new()), Ok(value), "{source}");
    }
}

#[test]
fn validates_arity_and_definition_target_before_side_effects() {
    for (source, expected) in [
        ("(-)", "'-' expects exactly 2 arguments"),
        ("(- 1)", "'-' expects exactly 2 arguments"),
        ("(- 1 2 3)", "'-' expects exactly 2 arguments"),
        ("(/)", "'/' expects exactly 2 arguments"),
        ("(/ 1)", "'/' expects exactly 2 arguments"),
        ("(/ 1 2 3)", "'/' expects exactly 2 arguments"),
        ("(define)", "'define' expects exactly 2 arguments"),
        ("(define x)", "'define' expects exactly 2 arguments"),
        ("(define x 1 2)", "'define' expects exactly 2 arguments"),
        ("(define 1 2)", "'define' target must be a symbol"),
        ("(define (x) 2)", "'define' target must be a symbol"),
        ("(- (define x 1))", "'-' expects exactly 2 arguments"),
        (
            "(define 1 (define x 1))",
            "'define' target must be a symbol",
        ),
    ] {
        let mut env = Env::new();
        assert_eq!(
            run_line(source, &mut env).unwrap_err(),
            expected,
            "{source}"
        );
        assert!(env.is_empty(), "{source}");
    }
}

#[test]
fn rejects_empty_non_symbol_and_unknown_operation_heads() {
    for source in ["()", "(1 2)", "((+ 1 2) 3)", "(unknown 1)", "(< 1 2)"] {
        assert!(run_line(source, &mut Env::new()).is_err(), "{source}");
    }
    let mut env = Env::new();
    assert_eq!(run_line("(define x 5)", &mut env), Ok(5.0));
    assert_eq!(
        run_line("(x 1)", &mut env).unwrap_err(),
        "unknown operation: x"
    );
    assert!(run_line("((define y 1) 2)", &mut env).is_err());
    assert!(run_line("(unknown (define y 1))", &mut env).is_err());
    assert!(!env.contains_key("y"));
}

#[test]
fn reassignment_happens_only_after_successful_rhs() {
    let mut env = Env::new();
    assert_eq!(run_line("(define x 10)", &mut env), Ok(10.0));
    assert_eq!(run_line("(define x (+ x 2))", &mut env), Ok(12.0));
    assert_eq!(
        run_line("(define x (/ 1 0))", &mut env).unwrap_err(),
        "division by zero"
    );
    assert_eq!(run_line("x", &mut env), Ok(12.0));
    assert!(run_line("(define absent unknown)", &mut env).is_err());
    assert!(!env.contains_key("absent"));
    assert!(run_line("(define x (* 1e308 1e308))", &mut env).is_err());
    assert_eq!(run_line("x", &mut env), Ok(12.0));
}

#[test]
fn all_arithmetic_arguments_are_evaluated_left_to_right() {
    let mut env = Env::new();
    assert_eq!(
        run_line("(+ (define x 1) (define x (+ x 1)) x)", &mut env),
        Ok(5.0)
    );
    assert_eq!(
        run_line("(* (define x 2) (define x (+ x 1)) x)", &mut env),
        Ok(18.0)
    );
    assert_eq!(
        run_line("(- (define x 8) (define x (+ x 2)))", &mut env),
        Ok(-2.0)
    );
    assert_eq!(
        run_line("(/ (define x 12) (define x (/ x 2)))", &mut env),
        Ok(2.0)
    );
    assert_eq!(run_line("x", &mut env), Ok(6.0));
    assert_eq!(run_line("(+ 1e16 -1e16 1)", &mut env), Ok(1.0));
    assert_eq!(run_line("(* 0 (define x 7))", &mut env), Ok(0.0));
    assert_eq!(run_line("x", &mut env), Ok(7.0));
}

#[test]
fn nested_definitions_survive_later_failure_but_later_arguments_do_not_run() {
    let mut env = Env::new();
    assert_eq!(run_line("(define x 10)", &mut env), Ok(10.0));
    assert!(run_line("(define x (+ (define y 3) (/ 1 0)))", &mut env).is_err());
    assert_eq!(env.get("x"), Some(&10.0));
    assert_eq!(env.get("y"), Some(&3.0));
    assert!(run_line("(define x (+ (define x 9) unknown))", &mut env).is_err());
    assert_eq!(env.get("x"), Some(&9.0));
    assert!(run_line("(+ unknown (define skipped 1))", &mut env).is_err());
    assert!(!env.contains_key("skipped"));
    assert!(run_line("(+ 1e308 1e308 (define skipped 1))", &mut env).is_err());
    assert!(!env.contains_key("skipped"));
    assert!(run_line("(define unparsed 1) 2", &mut env).is_err());
    assert!(!env.contains_key("unparsed"));
    assert_eq!(run_line("(+ 2 3)", &mut env), Ok(5.0));
}

#[test]
fn signed_zero_is_valid_but_never_a_valid_divisor() {
    let mut env = Env::new();
    for source in ["-0", "(+ -0)", "(* -0)", "(/ -0 2)", "(define z -0)", "z"] {
        assert_eq!(
            run_line(source, &mut env).unwrap().to_bits(),
            (-0.0_f64).to_bits(),
            "{source}"
        );
    }
    assert_eq!(
        run_line("+0", &mut env).unwrap().to_bits(),
        0.0_f64.to_bits()
    );
    for source in ["(/ 1 0)", "(/ 1 -0)", "(/ 0 -0)", "(/ 1 z)"] {
        assert_eq!(
            run_line(source, &mut env).unwrap_err(),
            "division by zero",
            "{source}"
        );
    }
}

#[test]
fn rejects_all_successfully_parsed_nonfinite_numbers() {
    for source in [
        "NaN",
        "nan",
        "-NaN",
        "inf",
        "-inf",
        "+infinity",
        "1e9999",
        "-1e9999",
    ] {
        assert!(!source.parse::<f64>().unwrap().is_finite());
        assert_eq!(
            parse(&tokenize(source)).unwrap_err(),
            "number must be finite",
            "{source}"
        );
        assert!(run_line(source, &mut Env::new()).is_err(), "{source}");
    }
}

#[test]
fn validates_manual_ast_numbers_and_directly_populated_environment_values() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut env = Env::from([("bad".to_owned(), value)]);
        assert_eq!(
            eval(&Expr::Number(value), &mut env).unwrap_err(),
            "number must be finite"
        );
        assert_eq!(
            run_line("bad", &mut env).unwrap_err(),
            "number must be finite"
        );
        assert!(run_line("(define x bad)", &mut env).is_err());
        assert!(!env.contains_key("x"));
        assert_eq!(run_line("1", &mut env), Ok(1.0));
    }
}

#[test]
fn rejects_nonfinite_intermediate_arithmetic_results() {
    for source in [
        "(+ 1e308 1e308)",
        "(+ 1e308 1e308 -1e308)",
        "(- -1e308 1e308)",
        "(* 1e308 1e308)",
        "(* 1e308 1e308 0)",
        "(/ 1e308 1e-308)",
    ] {
        assert_eq!(
            run_line(source, &mut Env::new()).unwrap_err(),
            "number must be finite",
            "{source}"
        );
    }
}

#[test]
fn accepts_finite_extremes_subnormals_and_underflow() {
    let mut env = Env::new();
    for value in [f64::MAX, f64::MIN, f64::MIN_POSITIVE, f64::from_bits(1)] {
        assert_eq!(run_line(&value.to_string(), &mut env), Ok(value));
        assert_eq!(eval(&Expr::Number(value), &mut env), Ok(value));
    }
    assert_eq!(run_line("(* 1e-300 1e-300)", &mut env), Ok(0.0));
    assert_eq!(run_line("1e-9999", &mut env), Ok(0.0));
    assert_eq!(
        run_line("(* -1e-300 1e-300)", &mut env).unwrap().to_bits(),
        (-0.0_f64).to_bits()
    );
}

#[test]
fn input_limit_counts_utf8_bytes_and_allows_the_exact_boundary() {
    let mut env = Env::new();
    let boundary = format!("{}1", " ".repeat(MAX_INPUT_BYTES - 1));
    assert_eq!(run_line(&boundary, &mut env), Ok(1.0));
    assert_eq!(
        run_line(&(boundary + " "), &mut env).unwrap_err(),
        "input exceeds byte limit"
    );
    let unicode = "é".repeat(MAX_INPUT_BYTES / 2);
    assert_eq!(unicode.len(), MAX_INPUT_BYTES);
    assert!(
        run_line(&unicode, &mut env)
            .unwrap_err()
            .starts_with("unknown variable:")
    );
    assert_eq!(
        run_line(&(unicode + "é"), &mut env).unwrap_err(),
        "input exceeds byte limit"
    );
    let source = format!("(define untouched 1){}", " ".repeat(MAX_INPUT_BYTES));
    assert_eq!(
        run_line(&source, &mut env).unwrap_err(),
        "input exceeds byte limit"
    );
    assert!(env.is_empty());
}

#[test]
fn direct_parser_entrypoints_enforce_token_count_and_aggregate_byte_limits() {
    let boundary = ["x".repeat(MAX_INPUT_BYTES)];
    assert_eq!(parse(&boundary), Ok(Expr::Symbol(boundary[0].clone())));
    let oversized = ["x".repeat(MAX_INPUT_BYTES + 1)];
    assert_eq!(parse(&oversized).unwrap_err(), "token bytes exceed limit");
    assert_eq!(
        parse_expr(&oversized, 0).unwrap_err(),
        "token bytes exceed limit"
    );
    assert_eq!(
        parse_list(&oversized, 0).unwrap_err(),
        "token bytes exceed limit"
    );
    let aggregate = [
        "x".repeat(MAX_INPUT_BYTES / 2),
        "y".repeat(MAX_INPUT_BYTES / 2 + 1),
    ];
    assert_eq!(parse(&aggregate).unwrap_err(), "token bytes exceed limit");
    let mut tokens = vec!["1".to_owned(); MAX_INPUT_BYTES];
    assert_eq!(parse(&tokens).unwrap_err(), "extra tokens after expression");
    tokens.push("1".to_owned());
    assert_eq!(parse(&tokens).unwrap_err(), "token count exceeds limit");
    assert_eq!(
        parse_expr(&tokens, 0).unwrap_err(),
        "token count exceeds limit"
    );
    assert_eq!(
        parse_list(&tokens, 1).unwrap_err(),
        "token count exceeds limit"
    );
}

fn nested_addition(depth: usize) -> String {
    format!("{}1{}", "(+ ".repeat(depth), ")".repeat(depth))
}

#[test]
fn parsing_and_evaluation_share_the_exact_list_depth_limit() {
    let mut env = Env::new();
    for depth in [0, 1, MAX_DEPTH - 1, MAX_DEPTH] {
        let source = nested_addition(depth);
        let expression = parse(&tokenize(&source)).unwrap();
        assert_eq!(eval(&expression, &mut env), Ok(1.0));
        assert_eq!(run_line(&source, &mut env), Ok(1.0));
    }
    let tokens = tokenize(&nested_addition(MAX_DEPTH + 1));
    assert_eq!(
        parse(&tokens).unwrap_err(),
        "maximum nesting depth exceeded"
    );
    assert!(parse_expr(&tokens, 0).is_err());
    assert!(parse_list(&tokens, 1).is_err());
}

#[test]
fn evaluation_limits_manually_constructed_trees() {
    let mut expression = Expr::Number(1.0);
    for _ in 0..MAX_DEPTH {
        expression = Expr::List(vec![Expr::Symbol("+".to_owned()), expression]);
    }
    assert_eq!(eval(&expression, &mut Env::new()), Ok(1.0));
    expression = Expr::List(vec![Expr::Symbol("+".to_owned()), expression]);
    assert_eq!(
        eval(&expression, &mut Env::new()).unwrap_err(),
        "maximum nesting depth exceeded"
    );
}

#[test]
fn variable_limit_allows_updates_but_blocks_new_names() {
    let mut env = Env::new();
    for index in 0..MAX_VARIABLES {
        assert_eq!(run_line(&format!("(define v{index} 1)"), &mut env), Ok(1.0));
    }
    assert_eq!(env.len(), MAX_VARIABLES);
    assert_eq!(run_line("(define v0 2)", &mut env), Ok(2.0));
    assert_eq!(
        run_line("(define extra 3)", &mut env).unwrap_err(),
        "maximum variable count exceeded"
    );
    assert!(!env.contains_key("extra"));
    assert_eq!(env.len(), MAX_VARIABLES);
    assert_eq!(run_line("(+ v0 v1)", &mut env), Ok(3.0));
}

#[test]
fn nested_definitions_cannot_bypass_the_variable_limit() {
    let mut env: Env = (0..MAX_VARIABLES - 1)
        .map(|index| (format!("v{index}"), 1.0))
        .collect();
    assert_eq!(
        run_line("(define outer (define inner 7))", &mut env).unwrap_err(),
        "maximum variable count exceeded"
    );
    assert_eq!(env.get("inner"), Some(&7.0));
    assert!(!env.contains_key("outer"));
    assert_eq!(env.len(), MAX_VARIABLES);
    assert_eq!(run_line("(define v0 (define inner 8))", &mut env), Ok(8.0));
    assert_eq!(env.get("inner"), Some(&8.0));
}

#[test]
fn externally_overfull_environments_still_allow_existing_name_updates() {
    let mut env: Env = (0..MAX_VARIABLES + 1)
        .map(|index| (format!("v{index}"), 1.0))
        .collect();
    assert_eq!(run_line("(define v0 3)", &mut env), Ok(3.0));
    assert!(run_line("(define extra 1)", &mut env).is_err());
    assert_eq!(env.len(), MAX_VARIABLES + 1);
}

#[test]
fn rejects_oversized_manual_symbols_without_echoing_them() {
    let huge = "é".repeat(MAX_INPUT_BYTES);
    let mut env = Env::new();
    let expressions = [
        Expr::Symbol(huge.clone()),
        Expr::List(vec![Expr::Symbol(huge.clone())]),
        Expr::List(vec![
            Expr::Symbol("define".to_owned()),
            Expr::Symbol(huge),
            Expr::Number(1.0),
        ]),
    ];
    for expression in expressions {
        assert_eq!(
            eval(&expression, &mut env).unwrap_err(),
            "symbol exceeds byte limit"
        );
    }
    assert!(env.is_empty());
}

#[test]
fn deterministic_generated_arithmetic_matches_direct_computation() {
    let mut env = Env::new();
    for left in -8..=8 {
        for right in -4..=4 {
            let source = format!("(+ (* {left} {right}) (- {left} {right}))");
            let expected = f64::from(left * right + left - right);
            assert_eq!(run_line(&source, &mut env), Ok(expected), "{source}");
            if right != 0 {
                let source = format!("(/ {left} {right})");
                let expected = f64::from(left) / f64::from(right);
                assert_eq!(run_line(&source, &mut env), Ok(expected), "{source}");
            }
        }
    }
    assert!(env.is_empty());
}
