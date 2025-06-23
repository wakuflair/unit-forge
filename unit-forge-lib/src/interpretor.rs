use std::collections::HashMap;

use chumsky::{extra::Err, prelude::*};

use crate::{DefinitionError, unit::UnitTable, unit_definition::UnitDefinitions};

pub type Error = (std::ops::Range<usize>, String);

#[derive(Debug)]
enum Expr<'src> {
    Num(f64, &'src str), // Store the unit as a string alongside the number
    Var(&'src str),

    Neg(Box<Expr<'src>>),
    Add(Box<Expr<'src>>, Box<Expr<'src>>),
    Sub(Box<Expr<'src>>, Box<Expr<'src>>),
    Mul(Box<Expr<'src>>, Box<Expr<'src>>),
    Div(Box<Expr<'src>>, Box<Expr<'src>>),

    Assign {
        name: &'src str,
        rhs: Box<Expr<'src>>,
    },

    To(Box<Expr<'src>>, Option<&'src str>),
}

pub struct Interpretor<'a> {
    unit_table: UnitTable<'a>,
    vars: HashMap<String, (f64, String)>,
}

impl<'a> Interpretor<'a> {
    pub fn new(unit_definitions: &'a UnitDefinitions) -> Result<Self, DefinitionError> {
        let unit_table = UnitTable::new(unit_definitions)?;
        Ok(Self {
            unit_table,
            vars: HashMap::new(),
        })
    }

    pub fn execute_command(&mut self, command: &str) -> Result<(f64, String), Vec<Error>> {
        let parsed =
            self.parser()
                .parse(command)
                .into_result()
                .map_err(|errs: Vec<Simple<'_, char>>| {
                    errs.into_iter()
                        .map(|err| (err.span().into_range(), err.to_string()))
                        .collect::<Vec<_>>()
                })?;
        let result = self
            .eval_expr(&parsed)
            .map_err(|err| vec![(0..command.len(), err)])?;

        self.vars.insert("$".to_string(), result.clone());

        Ok(result)
    }

    #[allow(clippy::let_and_return)]
    fn parser<'src>(&self) -> impl Parser<'src, &'src str, Expr<'src>, Err<Simple<'src, char>>> {
        let ident = text::ascii::ident().or(just("$")).padded();

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

        let assign = ident
            .then_ignore(just('='))
            .then(expr.clone())
            .map(|(name, rhs)| Expr::Assign {
                name,
                rhs: Box::new(rhs),
            });

        let to = expr
            .then(just(">>").padded().ignore_then(ident).or_not())
            .map(|(expr, unit)| Expr::To(Box::new(expr), unit));

        assign.or(to).padded()
    }

    fn eval_expr<'src>(&mut self, expr: &Expr<'src>) -> Result<(f64, String), String> {
        match expr {
            Expr::Num(num, unit_str) => match self.unit_table.base_units_map().get(unit_str) {
                Some(&(factor, base_unit)) => Ok(((*num * factor), base_unit.to_string())),
                None => Err(format!("Unknown unit: \"{}\"", unit_str)),
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
                        return Err(format!("Cannot evaluate {:?} {} {:?}", unit_a, op, unit_b));
                    }
                };
                if op == "*" {
                    Ok((val_a * val_b, new_unit))
                } else {
                    Ok((val_a / val_b, new_unit))
                }
            }
            Expr::Var(name) => {
                if let Some(val) = self.vars.get(*name) {
                    Ok((val.0, val.1.to_string()))
                } else {
                    Err(format!("Cannot find variable \"{name}\" in scope"))
                }
            }
            Expr::Assign { name, rhs } => {
                if *name == "$" {
                    return Err("Cannot assign to reserved variable \"$\"".to_string());
                }
                let rhs = self.eval_expr(rhs)?;
                self.vars.insert(name.to_string(), rhs.clone());
                Ok(rhs)
            }
            Expr::To(expr, unit) => {
                let (val, cur_unit) = self.eval_expr(expr)?;
                if let Some(unit_str) = unit {
                    if let Some(&(factor, base_unit)) =
                        self.unit_table.base_units_map().get(unit_str)
                    {
                        if cur_unit != base_unit {
                            Err(format!("Cannot convert to unit \"{}\"", *unit_str))
                        } else {
                            Ok((val / factor, unit_str.to_string()))
                        }
                    } else {
                        Err(format!("Unknown unit {}", unit_str))
                    }
                } else {
                    Ok((val, cur_unit))
                }
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
        let mut interceptor = Interpretor::new(&unit_definitions).unwrap();
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
        let mut interceptor = Interpretor::new(&unit_definitions).unwrap();
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

        let mut interceptor = Interpretor::new(&unit_definitions).unwrap();
        let result = interceptor.execute_command(expr);
        assert_eq!(result, Ok((7.0, "cm2".to_string())));
    }

    #[test]
    fn test_eval_incompatible_units() {
        let expr = "1 m + 2 sec";
        let unit_definitions = toml::from_str(
            r#"
[length]
m = { name = "meter", symbol = "m" }

[time]
sec = { name = "second", symbol = "s" }
"#,
        )
        .unwrap();

        let mut interceptor = Interpretor::new(&unit_definitions).unwrap();
        let result = interceptor.execute_command(expr);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors[0].1, "Cannot evaluate \"m\" + \"sec\"");
    }

    #[test]
    fn test_eval_invalid_unit_multiplication() {
        let expr = "2 m * 3 sec";
        let unit_definitions = toml::from_str(
            r#"
[length]
m = { name = "meter", symbol = "m" }

[time]
sec = { name = "second", symbol = "s" }
"#,
        )
        .unwrap();
        let mut interceptor = Interpretor::new(&unit_definitions).unwrap();
        let result = interceptor.execute_command(expr);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors[0].1, "Cannot evaluate \"m\" * \"sec\"");
    }

    #[test]
    fn test_eval_unknown_variable() {
        let expr = "x + 2";
        let unit_definitions = UnitDefinitions::default();
        let mut interceptor = Interpretor::new(&unit_definitions).unwrap();
        let result = interceptor.execute_command(expr);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors[0].1, "Cannot find variable \"x\" in scope");
    }

    #[test]
    fn test_eval_let_with_incompatible_units() {
        let unit_definitions = toml::from_str(
            r#"
[length]
m = { name = "meter", symbol = "m" }

[time]
sec = { name = "second", symbol = "s" }
"#,
        )
        .unwrap();
        let mut interceptor = Interpretor::new(&unit_definitions).unwrap();
        interceptor.execute_command("x = 5m").unwrap();
        let result = interceptor.execute_command("x + 3 sec");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors[0].1, "Cannot evaluate \"m\" + \"sec\"");
    }

    #[test]
    fn test_complex_expressions() {
        let str = std::fs::read_to_string("../unit_definitions/basic.ud").unwrap();
        let unit_definitions = toml::from_str(&str).unwrap();

        let mut interceptor = Interpretor::new(&unit_definitions).unwrap();
        interceptor.execute_command("x = 2 m").unwrap();
        interceptor.execute_command("y = 3 cm").unwrap();
        let result = interceptor.execute_command("x + y * 4").unwrap();
        assert_eq!(result, (2.12, "m".to_string()));

        let mut interceptor = Interpretor::new(&unit_definitions).unwrap();
        let result = interceptor.execute_command("((1km + 2cm) * 2 * 3m + 4cm2) * 5m + 6m3");
        assert_eq!(result, Ok((30006.602, "m3".to_string())));

        let mut interceptor = Interpretor::new(&unit_definitions).unwrap();
        let result = interceptor.execute_command("360 km / 2hour");
        assert_eq!(result, Ok((50.0, "mps".to_string())));
    }

    #[test]
    fn should_show_error_for_invalid_expression() {
        let unit_definitions = UnitDefinitions::default();
        let mut interceptor = Interpretor::new(&unit_definitions).unwrap();
        let result = interceptor.execute_command("1 + 2 *");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors[0].1, "found end of input at 7..7");
    }

    #[test]
    fn test_to_expr() {
        let expr = "1 m >> cm";
        let unit_definitions = toml::from_str(
            r#"
[length]
m = { name = "meter", symbol = "m" }
cm = { name = "centimeter", symbol = "cm", factor = 0.01 }
"#,
        )
        .unwrap();

        let mut interceptor = Interpretor::new(&unit_definitions).unwrap();
        let result = interceptor.execute_command(expr);
        assert_eq!(result, Ok((100.0, "cm".to_string())));
    }

    #[test]
    fn test_to_expr_with_incompatible_units() {
        let expr = "1 m >> sec";
        let unit_definitions = toml::from_str(
            r#"
[length]
m = { name = "meter", symbol = "m" }

[time]
sec = { name = "sec", symbol = "s" }
"#,
        )
        .unwrap();

        let mut interceptor = Interpretor::new(&unit_definitions).unwrap();
        let result = interceptor.execute_command(expr);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors[0].1, "Cannot convert to unit \"sec\"");
    }
}
