use chumsky::prelude::*;

use crate::unit_table::UnitTable;

#[derive(Debug)]
enum Expr<'src> {
    Num(f64, Option<&'src str>), // Store the unit as a string alongside the number
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

#[allow(clippy::let_and_return)]
fn parser<'src>() -> impl Parser<'src, &'src str, Expr<'src>> {
    let ident = text::ascii::ident().padded();

    let expr = recursive(|expr| {
        let int = text::int(10)
            .then(ident.or_not())
            .map(|(num, unit): (&str, Option<&str>)| Expr::Num(num.parse().unwrap(), unit));

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

fn eval<'src>(
    expr: &'src Expr<'src>,
    vars: &mut Vec<(&'src str, (f64, Option<&'src str>))>,
    unit_table: &'src UnitTable,
) -> Result<(f64, Option<&'src str>), String> {
    match expr {
        Expr::Num(num, unit_str) => {
            if let Some(u) = unit_str {
                match unit_table.base_units_map().get(u) {
                    Some(&(factor, base_unit)) => Ok(((*num * factor), Some(base_unit))),
                    None => Err(format!("Unknown unit: {}", u)),
                }
            } else {
                Ok((*num, *unit_str))
            }
        }
        Expr::Neg(a) => {
            let (val, unit) = eval(a, vars, unit_table)?;
            Ok((-val, unit))
        }
        Expr::Add(a, b) | Expr::Sub(a, b) => {
            let (val_a, unit_a) = eval(a, vars, unit_table)?;
            let (val_b, unit_b) = eval(b, vars, unit_table)?;

            match (unit_a, unit_b) {
                (None, None) => {
                    let op = if matches!(expr, Expr::Add(_, _)) {
                        "+"
                    } else {
                        "-"
                    };
                    Ok((
                        if op == "+" {
                            val_a + val_b
                        } else {
                            val_a - val_b
                        },
                        None,
                    ))
                }
                (Some(u_a), Some(u_b)) => {
                    // Find the category that contains both units
                    let category = unit_table
                        .unit_definitions()
                        .categories
                        .iter()
                        .find(|(_, units)| units.contains_key(u_a) && units.contains_key(u_b))
                        .map(|(cat, _)| cat);

                    if let Some(category) = category {
                        let units = &unit_table.unit_definitions().categories[category];
                        let unit_a = &units[u_a];
                        let unit_b = &units[u_b];

                        // Convert unit_b to unit_a
                        let converted_b = val_b * unit_b.factor / unit_a.factor;
                        let op = if matches!(expr, Expr::Add(_, _)) {
                            "+"
                        } else {
                            "-"
                        };
                        let result = if op == "+" {
                            val_a + converted_b
                        } else {
                            val_a - converted_b
                        };

                        Ok((result, Some(u_a)))
                    } else {
                        Err(format!("Incompatible units: {:?} and {:?}", u_a, u_b))
                    }
                }
                _ => Err("Cannot mix unitless and unit values".to_string()),
            }
        }
        Expr::Mul(a, b) | Expr::Div(a, b) => {
            let op = if matches!(expr, Expr::Mul(_, _)) {
                "*"
            } else {
                "/"
            };
            let (val_a, unit_a) = eval(a, vars, unit_table)?;
            let (val_b, unit_b) = eval(b, vars, unit_table)?;
            let new_unit = match (unit_a, unit_b) {
                (Some(u_a), Some(u_b)) => match unit_table.derived_units_map().get(&(u_a, op, u_b))
                {
                    Some(&new_unit) => Some(new_unit),
                    _ => return Err(format!("Cannot evaluate {:?} {} {:?}", u_a, op, u_b)),
                },
                (None, None) => None,
                _ => return Err(format!("Cannot evaluate {:?} {} {:?}", unit_a, op, unit_b)),
            };
            if op == "*" {
                Ok((val_a * val_b, new_unit))
            } else {
                Ok((val_a / val_b, new_unit))
            }
        }
        Expr::Var(name) => {
            if let Some((_, val)) = vars.iter().rev().find(|(var, _)| var == name) {
                Ok(*val)
            } else {
                Err(format!("Cannot find variable `{name}` in scope"))
            }
        }
        Expr::Let { name, rhs, then } => {
            let rhs = eval(rhs, vars, unit_table)?;
            vars.push((*name, rhs));
            let output = eval(then, vars, unit_table);
            vars.pop();
            output
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::UnitDefinitions;

    use super::*;

    #[test]
    fn test_eval_without_unit() {
        let expr = "1 + 2 * 3";
        let parsed = parser().parse(expr).unwrap();
        let mut vars = Vec::new();
        let unit_definitions = UnitDefinitions::default();
        let unit = UnitTable::new(&unit_definitions).unwrap();
        let result = eval(&parsed, &mut vars, &unit);
        assert_eq!(result, Ok((7.0, None)));
    }

    #[test]
    fn test_eval_with_unit_definitions() {
        let expr = "1 m + 2 cm";
        let parsed = parser().parse(expr).unwrap();
        let mut vars = Vec::new();
        let unit_definitions = toml::from_str(
            r#"
[length]
m = { name = "meter", symbol = "m" }
cm = { name = "centimeter", symbol = "cm", factor = 0.01 }
"#,
        )
        .unwrap();
        let unit = UnitTable::new(&unit_definitions).unwrap();
        let result = eval(&parsed, &mut vars, &unit);
        assert_eq!(result, Ok((1.02, Some("m"))),);
    }

    #[test]
    fn test_eval_with_unit_map() {
        let expr = "1 cm2 + 2 cm * 3cm";
        let parsed = parser().parse(expr).unwrap();
        let mut vars = Vec::new();
        let unit_definitions = toml::from_str(
            r#"
[length]
cm = { name = "center meter", symbol = "cm" }

[area]
cm2 = { name = "square center meter", symbol = "cm2", derived = "cm * cm" }
"#,
        )
        .unwrap();

        let unit = UnitTable::new(&unit_definitions).unwrap();
        let result = eval(&parsed, &mut vars, &unit);
        assert_eq!(result, Ok((7.0, Some("cm2"))),);
    }

    #[test]
    fn test_eval_incompatible_units_add() {
        let expr = "1 m + 2 s";
        let parsed = parser().parse(expr).unwrap();
        let mut vars = Vec::new();
        let unit_definitions = UnitDefinitions::default();
        let unit = UnitTable::new(&unit_definitions).unwrap();
        let result = eval(&parsed, &mut vars, &unit);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Incompatible units"));
    }

    #[test]
    fn test_eval_incompatible_units_sub() {
        let expr = "5 kg - 2 m";
        let parsed = parser().parse(expr).unwrap();
        let mut vars = Vec::new();
        let unit_definitions = UnitDefinitions::default();
        let unit = UnitTable::new(&unit_definitions).unwrap();
        let result = eval(&parsed, &mut vars, &unit);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Incompatible units"));
    }

    #[test]
    fn test_eval_invalid_unit_multiplication() {
        let expr = "2 m * 3 s"; // assuming m*s is not defined in default unit definitions
        let parsed = parser().parse(expr).unwrap();
        let mut vars = Vec::new();
        let unit_definitions = UnitDefinitions::default();
        let unit = UnitTable::new(&unit_definitions).unwrap();
        let result = eval(&parsed, &mut vars, &unit);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cannot evaluate"));
    }

    #[test]
    fn test_eval_unknown_variable() {
        let expr = "x + 2";
        let parsed = parser().parse(expr).unwrap();
        let mut vars = Vec::new();
        let unit_definitions = UnitDefinitions::default();
        let unit = UnitTable::new(&unit_definitions).unwrap();
        let result = eval(&parsed, &mut vars, &unit);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cannot find variable `x`"));
    }

    #[test]
    fn test_eval_let_with_incompatible_units() {
        let expr = "let x = 5 m; x + 3 s";
        let parsed = parser().parse(expr).unwrap();
        let mut vars = Vec::new();
        let unit_definitions = UnitDefinitions::default();
        let unit = UnitTable::new(&unit_definitions).unwrap();
        let result = eval(&parsed, &mut vars, &unit);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Incompatible units"));
    }
}
