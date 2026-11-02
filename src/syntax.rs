use crate::{MAX_DEPTH, MAX_INPUT_BYTES};

/// An owned Lisp expression.
///
/// Parsing produces only finite numbers. Because callers may construct these
/// variants directly, evaluation validates numbers and nesting independently.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A floating-point literal.
    Number(f64),
    /// A variable name or an operation name in list-head position.
    Symbol(String),
    /// An ordered sequence of expressions, including its operation head.
    List(Vec<Expr>),
}

/// Split UTF-8 text on Unicode whitespace and isolate ASCII parentheses.
///
/// Other characters are preserved, including signs, punctuation, and case.
/// This infallible scanner does not impose resource limits; use [`crate::run_line`]
/// for bounded text input or [`parse`] to validate externally supplied tokens.
#[must_use]
pub fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut start = None;
    for (offset, character) in input.char_indices() {
        if character.is_whitespace() || matches!(character, '(' | ')') {
            if let Some(begin) = start.take() {
                tokens.push(input[begin..offset].to_owned());
            }
            if matches!(character, '(' | ')') {
                tokens.push(character.to_string());
            }
        } else if start.is_none() {
            start = Some(offset);
        }
    }
    if let Some(begin) = start {
        tokens.push(input[begin..].to_owned());
    }
    tokens
}

/// Parse exactly one expression, rejecting any trailing tokens.
///
/// # Errors
///
/// Rejects empty input, malformed parentheses, invalid externally supplied
/// tokens, non-finite numbers, and nesting beyond [`MAX_DEPTH`]. Before AST
/// allocation, limits both token count and aggregate UTF-8 token bytes to
/// [`MAX_INPUT_BYTES`]. A token is one parenthesis or a nonempty atom without
/// whitespace or parentheses. Empty lists are syntax trees but cannot evaluate.
/// Tokens that fail `parse::<f64>()` become symbols rather than syntax errors.
pub fn parse(tokens: &[String]) -> Result<Expr, String> {
    validate_tokens(tokens)?;
    let (expression, next) = parse_expr_at(tokens, 0, 0)?;
    if next != tokens.len() {
        return Err("extra tokens after expression".to_owned());
    }
    Ok(expression)
}

/// Parse one expression at `pos`, returning the next **unconsumed** cursor.
///
/// Does not require consuming the suffix, so callers can parse adjacent forms.
/// The selected expression starts at depth zero. Validates the complete token
/// slice against the same byte, token-count, and token-shape bounds as [`parse`].
///
/// # Errors
///
/// Returns an error for an out-of-range cursor, invalid tokens or syntax,
/// non-finite numbers, or excessive nesting. The input is never mutated.
pub fn parse_expr(tokens: &[String], pos: usize) -> Result<(Expr, usize), String> {
    validate_tokens(tokens)?;
    parse_expr_at(tokens, pos, 0)
}

/// Parse a list starting immediately **after** its opening `(` token.
///
/// Returns the list and the cursor immediately after its matching `)`.
/// The enclosing list counts as depth one. Validates the complete token slice
/// using the same bounds as [`parse`]; `start` must follow an actual `(`.
///
/// # Errors
///
/// Rejects invalid cursors, invalid tokens, missing closing parentheses,
/// non-finite numbers, and excessive nesting. The input is never mutated.
pub fn parse_list(tokens: &[String], start: usize) -> Result<(Expr, usize), String> {
    validate_tokens(tokens)?;
    let previous = start.checked_sub(1).and_then(|pos| tokens.get(pos));
    if previous.map(String::as_str) != Some("(") {
        return Err("list start must follow '('".to_owned());
    }
    parse_list_at(tokens, start, 1)
}

fn validate_tokens(tokens: &[String]) -> Result<(), String> {
    if tokens.len() > MAX_INPUT_BYTES {
        return Err("token count exceeds limit".to_owned());
    }
    let mut remaining = MAX_INPUT_BYTES;
    for token in tokens {
        if token.len() > remaining {
            return Err("token bytes exceed limit".to_owned());
        }
        remaining -= token.len();
        if token.is_empty()
            || (token != "("
                && token != ")"
                && token
                    .chars()
                    .any(|character| character.is_whitespace() || matches!(character, '(' | ')')))
        {
            return Err("invalid token".to_owned());
        }
    }
    Ok(())
}

fn parse_expr_at(tokens: &[String], pos: usize, depth: usize) -> Result<(Expr, usize), String> {
    let token = tokens
        .get(pos)
        .ok_or_else(|| "unexpected end of input".to_owned())?;
    match token.as_str() {
        "(" => {
            if depth >= MAX_DEPTH {
                return Err("maximum nesting depth exceeded".to_owned());
            }
            parse_list_at(tokens, pos + 1, depth + 1)
        }
        ")" => Err("unexpected ')'".to_owned()),
        _ => {
            let expression = match token.parse::<f64>() {
                Ok(number) if number.is_finite() => Expr::Number(number),
                Ok(_) => return Err("number must be finite".to_owned()),
                Err(_) => Expr::Symbol(token.clone()),
            };
            Ok((expression, pos + 1))
        }
    }
}

fn parse_list_at(tokens: &[String], start: usize, depth: usize) -> Result<(Expr, usize), String> {
    let mut expressions = Vec::new();
    let mut pos = start;
    loop {
        match tokens.get(pos).map(String::as_str) {
            Some(")") => return Ok((Expr::List(expressions), pos + 1)),
            None => return Err("missing ')'".to_owned()),
            Some(_) => {
                let (expression, next) = parse_expr_at(tokens, pos, depth)?;
                expressions.push(expression);
                pos = next;
            }
        }
    }
}
