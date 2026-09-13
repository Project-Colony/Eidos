//! Checked implementation of OMODFramework 05b3d562's ordered expression
//! reductions (erri120, GPL-3.0-only). Operator order is compatibility data,
//! including division before multiplication and addition before subtraction.
use super::{err, float, number, ObmmError, MAX_DEPTH};

pub(super) fn evaluate(tokens: &[String], integer: bool, line: usize) -> Result<String, ObmmError> {
    fn recurse(
        tokens: &[String],
        integer: bool,
        line: usize,
        depth: usize,
    ) -> Result<String, ObmmError> {
        if depth > MAX_DEPTH {
            return Err(err(line, "expression depth exceeded"));
        }
        let mut flat = Vec::new();
        let mut i = 0;
        while i < tokens.len() {
            if tokens[i] == "(" {
                let start = i + 1;
                let mut level = 1;
                i += 1;
                while i < tokens.len() && level > 0 {
                    match tokens[i].as_str() {
                        "(" => level += 1,
                        ")" => level -= 1,
                        _ => {}
                    }
                    if level > 0 {
                        i += 1;
                    }
                }
                if level != 0 {
                    return Err(err(line, "unclosed expression parenthesis"));
                }
                flat.push(recurse(&tokens[start..i], integer, line, depth + 1)?);
            } else if tokens[i] == ")" {
                return Err(err(line, "unmatched expression parenthesis"));
            } else {
                flat.push(tokens[i].clone());
            }
            i += 1;
        }
        let ops: &[&str] = if integer {
            &[
                "not", "and", "or", "xor", "mod", "%", "^", "/", "*", "+", "-",
            ]
        } else {
            &[
                "sin", "cos", "tan", "sinh", "cosh", "tanh", "exp", "log", "ln", "mod", "%", "^",
                "/", "*", "+", "-",
            ]
        };
        for op in ops {
            let unary = matches!(
                *op,
                "not" | "sin" | "cos" | "tan" | "sinh" | "cosh" | "tanh" | "exp" | "log" | "ln"
            );
            while let Some(i) = flat.iter().position(|t| t == op) {
                if i + 1 >= flat.len() || (!unary && i == 0) {
                    return Err(err(line, "expression operator is missing an operand"));
                }
                let start = if unary { i } else { i - 1 };
                let left = &flat[if unary { i + 1 } else { i - 1 }];
                let right = &flat[i + 1];
                let value = if integer {
                    let a = number::<i32>(left, line)?;
                    let b = if unary {
                        0
                    } else {
                        number::<i32>(right, line)?
                    };
                    let result = match *op {
                        "not" => Some(!a),
                        "and" => Some(a & b),
                        "or" => Some(a | b),
                        "xor" => Some(a ^ b),
                        "mod" | "%" => a.checked_rem(b),
                        "/" => a.checked_div(b),
                        "*" => a.checked_mul(b),
                        "+" => a.checked_add(b),
                        "-" => a.checked_sub(b),
                        "^" => {
                            let value = f64::from(a).powi(b);
                            (value.is_finite()
                                && value >= f64::from(i32::MIN)
                                && value <= f64::from(i32::MAX))
                            .then_some(value as i32)
                        }
                        _ => None,
                    }
                    .ok_or_else(|| err(line, "integer overflow or division by zero"))?;
                    result.to_string()
                } else {
                    let a = float(left, line)?;
                    let b = if unary { 0.0 } else { float(right, line)? };
                    let result = match *op {
                        "sin" => a.sin(),
                        "cos" => a.cos(),
                        "tan" => a.tan(),
                        "sinh" => a.sinh(),
                        "cosh" => a.cosh(),
                        "tanh" => a.tanh(),
                        "exp" => a.exp(),
                        "log" => a.log10(),
                        "ln" => a.ln(),
                        "mod" | "%" => a % b,
                        "^" => a.powf(b),
                        "/" => a / b,
                        "*" => a * b,
                        "+" => a + b,
                        "-" => a - b,
                        _ => return Err(err(line, "unknown numeric operator")),
                    };
                    if !result.is_finite() {
                        return Err(err(line, "non-finite expression result"));
                    }
                    result.to_string()
                };
                flat.splice(start..=i + 1, [value]);
            }
        }
        if flat.len() != 1 {
            return Err(err(line, "expression must produce exactly one value"));
        }
        if integer {
            Ok(number::<i32>(&flat[0], line)?.to_string())
        } else {
            Ok(float(&flat[0], line)?.to_string())
        }
    }
    recurse(tokens, integer, line, 0)
}
