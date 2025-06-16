use std::collections::HashMap;

use chumsky::prelude::*;

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
    unit_map: &HashMap<(&'src str, &'src str, &'src str), &'src str>,
) -> Result<(f64, Option<&'src str>), String> {
    match expr {
        Expr::Num(num, unit) => Ok((*num, *unit)),
        Expr::Neg(a) => {
            let (val, unit) = eval(a, vars, unit_map)?;
            Ok((-val, unit))
        }
        Expr::Add(a, b) => {
            let (val_a, unit_a) = eval(a, vars, unit_map)?;
            let (val_b, unit_b) = eval(b, vars, unit_map)?;
            if unit_a == unit_b {
                Ok((val_a + val_b, unit_a))
            } else {
                Err(format!("Incompatible units: {:?} and {:?}", unit_a, unit_b))
            }
        }
        Expr::Sub(a, b) => {
            let (val_a, unit_a) = eval(a, vars, unit_map)?;
            let (val_b, unit_b) = eval(b, vars, unit_map)?;
            if unit_a == unit_b {
                Ok((val_a - val_b, unit_a))
            } else {
                Err(format!("Incompatible units: {:?} and {:?}", unit_a, unit_b))
            }
        }
        Expr::Mul(a, b) | Expr::Div(a, b) => {
            let op = if matches!(expr, Expr::Mul(_, _)) {
                "*"
            } else {
                "/"
            };
            let (val_a, unit_a) = eval(a, vars, unit_map)?;
            let (val_b, unit_b) = eval(b, vars, unit_map)?;
            let new_unit = match (unit_a, unit_b) {
                (Some(u_a), Some(u_b)) => match unit_map.get(&(u_a, op, u_b)) {
                    Some(&new_unit) => Some(new_unit),
                    _ => return Err(format!("Cannot evaluate {:?} {} {:?}", unit_a, op, unit_b)),
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
            let rhs = eval(rhs, vars, unit_map)?;
            vars.push((*name, rhs));
            let output = eval(then, vars, unit_map);
            vars.pop();
            output
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_without_unit() {
        let expr = "1 + 2 * 3";
        let parsed = parser().parse(expr).unwrap();
        let mut vars = Vec::new();
        let map = HashMap::new();
        let result = eval(&parsed, &mut vars, &map);
        assert_eq!(result, Ok((7.0, None)));
    }

    #[test]
    fn test_eval_with_unit() {
        let expr = "1 cm2 + 2 cm * 3cm";
        let parsed = parser().parse(expr).unwrap();
        let mut vars = Vec::new();
        let map = HashMap::from([(("cm", "*", "cm"), "cm2")]);
        let result = eval(&parsed, &mut vars, &map);
        assert_eq!(result, Ok((7.0, Some("cm2"))),);
    }
}
