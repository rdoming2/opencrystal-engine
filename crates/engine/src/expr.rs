use rand::RngExt;
use std::collections::HashMap;

pub fn eval_expression(expr: &str, vars: &HashMap<String, f64>) -> Result<f64, String> {
    let mut parser = Parser::new(expr, vars);
    let value = parser.parse_expr()?;
    parser.skip_whitespace();
    if parser.peek().is_some() {
        return Err(format!("unexpected input at {}", parser.pos));
    }
    Ok(value)
}

struct Parser<'a> {
    chars: Vec<char>,
    pos: usize,
    vars: &'a HashMap<String, f64>,
}

impl<'a> Parser<'a> {
    fn new(expr: &str, vars: &'a HashMap<String, f64>) -> Self {
        Self {
            chars: expr.chars().collect(),
            pos: 0,
            vars,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<char> {
        if self.pos >= self.chars.len() {
            return None;
        }
        let ch = self.chars[self.pos];
        self.pos += 1;
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(ch) if ch.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn parse_expr(&mut self) -> Result<f64, String> {
        let mut value = self.parse_term()?;
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some('+') => {
                    self.next();
                    value += self.parse_term()?;
                }
                Some('-') => {
                    self.next();
                    value -= self.parse_term()?;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    fn parse_term(&mut self) -> Result<f64, String> {
        let mut value = self.parse_factor()?;
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some('*') => {
                    self.next();
                    value *= self.parse_factor()?;
                }
                Some('/') => {
                    self.next();
                    let denom = self.parse_factor()?;
                    if denom == 0.0 {
                        return Err("division by zero".to_string());
                    }
                    value /= denom;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    fn parse_factor(&mut self) -> Result<f64, String> {
        self.skip_whitespace();
        match self.peek() {
            Some('-') => {
                self.next();
                Ok(-self.parse_factor()?)
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<f64, String> {
        self.skip_whitespace();
        match self.peek() {
            Some('(') => {
                self.next();
                let value = self.parse_expr()?;
                self.skip_whitespace();
                if self.next() != Some(')') {
                    return Err("expected ')'".to_string());
                }
                Ok(value)
            }
            Some(ch) if ch.is_ascii_digit() || ch == '.' => self.parse_number(),
            Some(ch) if ch.is_ascii_alphabetic() || ch == '_' => self.parse_identifier(),
            Some(other) => Err(format!("unexpected '{}'", other)),
            None => Err("unexpected end of input".to_string()),
        }
    }

    fn parse_number(&mut self) -> Result<f64, String> {
        let start = self.pos;
        let mut has_dot = false;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.next();
            } else if ch == '.' && !has_dot {
                has_dot = true;
                self.next();
            } else {
                break;
            }
        }
        let slice: String = self.chars[start..self.pos].iter().collect();
        slice
            .parse::<f64>()
            .map_err(|_| format!("invalid number '{}'", slice))
    }

    fn parse_identifier(&mut self) -> Result<f64, String> {
        let ident = self.parse_ident_string();
        self.skip_whitespace();
        if self.peek() == Some('(') {
            self.next();
            let args = self.parse_args()?;
            self.apply_function(&ident, &args)
        } else {
            self.vars
                .get(&ident)
                .copied()
                .ok_or_else(|| format!("unknown variable '{}'", ident))
        }
    }

    fn parse_ident_string(&mut self) -> String {
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' {
                self.next();
            } else {
                break;
            }
        }
        self.chars[start..self.pos].iter().collect()
    }

    fn parse_args(&mut self) -> Result<Vec<f64>, String> {
        let mut args = Vec::new();
        loop {
            self.skip_whitespace();
            if self.peek() == Some(')') {
                self.next();
                break;
            }
            let value = self.parse_expr()?;
            args.push(value);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.next();
                }
                Some(')') => {
                    self.next();
                    break;
                }
                _ => return Err("expected ',' or ')'".to_string()),
            }
        }
        Ok(args)
    }

    fn apply_function(&self, name: &str, args: &[f64]) -> Result<f64, String> {
        match name {
            "RAND" => {
                if args.len() != 2 {
                    return Err("RAND expects 2 arguments".to_string());
                }
                let min = args[0];
                let max = args[1];
                let mut rng = rand::rng();
                Ok(rng.random_range(min..=max))
            }
            "ROUND" => {
                if args.len() != 1 {
                    return Err("ROUND expects 1 argument".to_string());
                }
                Ok(args[0].round())
            }
            "FLOOR" => {
                if args.len() != 1 {
                    return Err("FLOOR expects 1 argument".to_string());
                }
                Ok(args[0].floor())
            }
            "CEIL" => {
                if args.len() != 1 {
                    return Err("CEIL expects 1 argument".to_string());
                }
                Ok(args[0].ceil())
            }
            _ => Err(format!("unknown function '{}'", name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::eval_expression;
    use std::collections::HashMap;

    #[test]
    fn evaluates_precedence_and_parentheses() {
        let vars = HashMap::new();
        assert_eq!(eval_expression("1 + 2 * 3", &vars), Ok(7.0));
        assert_eq!(eval_expression("(1 + 2) * 3", &vars), Ok(9.0));
    }

    #[test]
    fn evaluates_unary_and_variables() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), 4.0);
        assert_eq!(eval_expression("-(-2)", &vars), Ok(2.0));
        assert_eq!(eval_expression("x * -2", &vars), Ok(-8.0));
    }

    #[test]
    fn evaluates_builtin_functions() {
        let vars = HashMap::new();
        assert_eq!(eval_expression("ROUND(1.6)", &vars), Ok(2.0));
        assert_eq!(eval_expression("FLOOR(1.9)", &vars), Ok(1.0));
        assert_eq!(eval_expression("CEIL(1.1)", &vars), Ok(2.0));
    }

    #[test]
    fn rand_returns_value_within_bounds() {
        let vars = HashMap::new();
        for _ in 0..64 {
            let value = eval_expression("RAND(3, 5)", &vars).expect("RAND should parse");
            assert!(value >= 3.0);
            assert!(value <= 5.0);
        }
    }

    #[test]
    fn reports_parse_and_eval_errors() {
        let vars = HashMap::new();
        assert!(eval_expression("unknown + 1", &vars)
            .expect_err("unknown variable should fail")
            .contains("unknown variable"));
        assert_eq!(
            eval_expression("1 / 0", &vars).expect_err("divide by zero should fail"),
            "division by zero"
        );
        assert_eq!(
            eval_expression("(1 + 2", &vars).expect_err("missing parenthesis should fail"),
            "expected ')'"
        );
        assert_eq!(
            eval_expression("ROUND(1, 2)", &vars).expect_err("arity mismatch should fail"),
            "ROUND expects 1 argument"
        );
    }
}
