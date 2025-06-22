use chumsky::prelude::*;

use crate::{unit::UnitTable, unit_definition::UnitDefinitions, DefinitionError};

#[derive(Debug)]
enum Expr<'src> {
    Num(f64, &'src str), // Store the unit as a string alongside the number
    Var(&'src str),

    Neg(Box<Expr<'src>>),
    Add(Box<Expr<'src>>, Box<Expr<'src>>),
    Sub(Box<Expr<'src>>, Box<Expr<'src>>),
    Mul(Box<Expr<'src>>, Box<Expr<'src>>),
    Div(Box<Expr<'src>>, Box<Expr<'src>>),

    Let {
        name: &'src str,
        rhs: Box<Expr<'src>>,
        then: Box<Expr<'src>>,
    },
}

pub struct Interceptor<'a> {
    unit_table: UnitTable<'a>,
    vars: Vec<(String, (f64, String))>,
}

impl<'a> Interceptor<'a> {
    pub fn new(unit_definitions: &'a UnitDefinitions) -> Result<Self, DefinitionError> {
        let unit_table = UnitTable::new(unit_definitions)?;
        Ok(Self {
            unit_table,
            vars: Vec::new(),
        })
    }

    pub fn execute_command(&mut self, command: &'a str) -> Result<(f64, String), String> {
        let parsed = self
            .parser()
            .parse(command)
            .into_result()
            .map_err(|_| "Parsing failed".to_string())?;
        self.eval_expr(&parsed)
    }

    #[allow(clippy::let_and_return)]
    fn parser(&self) -> impl Parser<'a, &'a str, Expr<'a>> {
        let ident = text::ascii::ident().padded();

        let expr = recursive(|expr| {
            let int =
                text::int(10)
                    .then(ident.or_not())
                    .map(|(num, unit): (&str, Option<&str>)| {
                        Expr::Num(num.parse().unwrap(), unit.unwrap_or("")) // Default to empty unit if no unit is provided
                    });

            let atom = int
                .or(expr.delimited_by(just('('), just(')')))
                .or(ident.map(Expr::Var))
                .padded();

            let op = |c| just(c).padded();

            let unary = op('-')
                .repeated()
                .foldr(atom, |_op, rhs| Expr::Neg(Box::new(rhs)));

            let product = unary.clone().foldl(
                choice((
                    op('*').to(Expr::Mul as fn(_, _) -> _),
                    op('/').to(Expr::Div as fn(_, _) -> _),
                ))
                .then(unary)
                .repeated(),
                |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
            );

            let sum = product.clone().foldl(
                choice((
                    op('+').to(Expr::Add as fn(_, _) -> _),
                    op('-').to(Expr::Sub as fn(_, _) -> _),
                ))
                .then(product)
                .repeated(),
                |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
            );

            sum
        });

        let decl = recursive(|decl| {
            let r#let = text::ascii::keyword("let")
                .ignore_then(ident)
                .then_ignore(just('='))
                .then(expr.clone())
                .then_ignore(just(';'))
                .then(decl.clone())
                .map(|((name, rhs), then)| Expr::Let {
                    name,
                    rhs: Box::new(rhs),
                    then: Box::new(then),
                });

            r#let.or(expr).padded()
        });

        decl
    }

    fn eval_expr(&mut self, expr: &Expr<'a>) -> Result<(f64, String), String> {
        match expr {
            Expr::Num(num, unit_str) => match self.unit_table.base_units_map().get(unit_str) {
                Some(&(factor, base_unit)) => Ok(((*num * factor), base_unit.to_string())),
                None => Err(format!("Unknown unit: {}", unit_str)),
            },
            Expr::Neg(a) => {
                let (val, unit) = self.eval_expr(a)?;
                Ok((-val, unit))
            }
            Expr::Add(a, b) | Expr::Sub(a, b) => {
                let (val_a, unit_a) = self.eval_expr(a)?;
                let (val_b, unit_b) = self.eval_expr(b)?;

                let op = if matches!(expr, Expr::Add(_, _)) {
                    "+"
                } else {
                    "-"
                };
                if unit_a != unit_b {
                    return Err(format!("Cannot evaluate {:?} {} {:?}", unit_a, op, unit_b));
                }
                let result = if op == "+" {
                    val_a + val_b
                } else {
                    val_a - val_b
                };

                Ok((result, unit_b))
            }
            Expr::Mul(a, b) | Expr::Div(a, b) => {
                let (val_a, unit_a) = self.eval_expr(a)?;
                let (val_b, unit_b) = self.eval_expr(b)?;

                let op = if matches!(expr, Expr::Mul(_, _)) {
                    "*"
                } else {
                    "/"
                };
                let new_unit = match self.unit_table.derived_units_map().get(&(
                    unit_a.as_ref(),
                    op,
                    unit_b.as_ref(),
                )) {
                    Some(&new_unit) => new_unit.to_string(),
                    None if unit_a.is_empty() => unit_b,
                    None if unit_b.is_empty() => unit_a,
                    None => {
                        return Err(format!("Cannot evaluate {:?} {} {:?}", unit_a, op, unit_b))
                    }
                };
                if op == "*" {
                    Ok((val_a * val_b, new_unit))
                } else {
                    Ok((val_a / val_b, new_unit))
                }
            }
            Expr::Var(name) => {
                if let Some((_, val)) = self.vars.iter().rev().find(|(var, _)| var == name) {
                    Ok((val.0, val.1.to_string()))
                } else {
                    Err(format!("Cannot find variable `{name}` in scope"))
                }
            }
            Expr::Let { name, rhs, then } => {
                let rhs = self.eval_expr(rhs)?;
                self.vars.push((name.to_string(), rhs));
                let output = self.eval_expr(then);
                self.vars.pop();
                output
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::unit_definition::UnitDefinitions;

    use super::*;

    #[test]
    fn test_eval_without_unit() {
        let expr = "1 + 2 * 3";
        let unit_definitions = UnitDefinitions::default();
        let mut interceptor = Interceptor::new(&unit_definitions).unwrap();
        let result = interceptor.execute_command(expr);
        assert_eq!(result, Ok((7.0, "".to_string())));
    }

    #[test]
    fn test_eval_with_unit_definitions() {
        let expr = "1 m + 2 cm";
        let unit_definitions = toml::from_str(
            r#"
[length]
m = { name = "meter", symbol = "m" }
cm = { name = "centimeter", symbol = "cm", factor = 0.01 }
"#,
        )
        .unwrap();
        let mut interceptor = Interceptor::new(&unit_definitions).unwrap();
        let result = interceptor.execute_command(expr);
        assert_eq!(result, Ok((1.02, "m".to_string())));
    }

    #[test]
    fn test_eval_with_unit_map() {
        let expr = "1 cm2 + 2 cm * 3cm";
        let unit_definitions = toml::from_str(
            r#"
[length]
cm = { name = "center meter", symbol = "cm" }

[area]
cm2 = { name = "square center meter", symbol = "cm2", derived = "cm * cm" }
"#,
        )
        .unwrap();

        let mut interceptor = Interceptor::new(&unit_definitions).unwrap();
        let result = interceptor.execute_command(expr);
        assert_eq!(result, Ok((7.0, "cm2".to_string())));
    }

    #[test]
    fn test_eval_incompatible_units() {
        let expr = "1 m + 2 second";
        let unit_definitions = toml::from_str(
            r#"
[length]
m = { name = "meter", symbol = "m" }

[time]
second = { name = "second", symbol = "s" }
"#,
        )
        .unwrap();

        let mut interceptor = Interceptor::new(&unit_definitions).unwrap();
        let result = interceptor.execute_command(expr);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Cannot evaluate \"m\" + \"second\"");
    }

    #[test]
    fn test_eval_invalid_unit_multiplication() {
        let expr = "2 m * 3 second";
        let unit_definitions = toml::from_str(
            r#"
[length]
m = { name = "meter", symbol = "m" }

[time]
second = { name = "second", symbol = "s" }
"#,
        )
        .unwrap();
        let mut interceptor = Interceptor::new(&unit_definitions).unwrap();
        let result = interceptor.execute_command(expr);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Cannot evaluate \"m\" * \"second\"");
    }

    #[test]
    fn test_eval_unknown_variable() {
        let expr = "x + 2";
        let unit_definitions = UnitDefinitions::default();
        let mut interceptor = Interceptor::new(&unit_definitions).unwrap();
        let result = interceptor.execute_command(expr);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cannot find variable `x`"));
    }

    #[test]
    fn test_eval_let_with_incompatible_units() {
        let expr = "let x = 5 m; x + 3 second";
        let unit_definitions = toml::from_str(
            r#"
[length]
m = { name = "meter", symbol = "m" }

[time]
second = { name = "second", symbol = "s" }
"#,
        )
        .unwrap();
        let mut interceptor = Interceptor::new(&unit_definitions).unwrap();
        let result = interceptor.execute_command(expr);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Cannot evaluate \"m\" + \"second\"");
    }

    #[test]
    fn test_complex_expressions() {
        let str = std::fs::read_to_string("../unit_definitions/basic.ud").unwrap();
        let unit_definitions = toml::from_str(&str).unwrap();

        let mut interceptor = Interceptor::new(&unit_definitions).unwrap();
        let result = interceptor
            .execute_command("let x = 2 m; let y = 3 cm; x + y * 4")
            .unwrap();
        assert_eq!(result, (2.12, "m".to_string()));

        let mut interceptor = Interceptor::new(&unit_definitions).unwrap();
        let result = interceptor.execute_command("((1km + 2cm) * 2 * 3m + 4cm2) * 5m + 6m3");
        assert_eq!(result, Ok((30006.602, "m3".to_string())));

        let mut interceptor = Interceptor::new(&unit_definitions).unwrap();
        let result = interceptor.execute_command("360 km / 2hour");
        assert_eq!(result, Ok((50.0, "mps".to_string())));
    }
}
