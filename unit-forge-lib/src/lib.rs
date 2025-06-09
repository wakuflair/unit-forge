use std::collections::HashMap;

mod units;
use thiserror::Error;
pub use units::*;

#[derive(Debug, Error)]
pub enum DefinitionError {
    #[error("Unit '{0}' is already defined in category '{1}'")]
    DuplicatedUnit(String, String),
    #[error("Derived unit '{0}' not found in derived expression '{1}'")]
    UnitNotFound(String, String),
    #[error("Invalid derived expression format: '{0}'")]
    InvalidDerivedExpression(String),
}

pub fn construct_unit_translation_map(
    definitions: &UnitDefinitions,
) -> Result<HashMap<(&str, &str, &str), &str>, DefinitionError> {
    // (unit_key, op, unit_key) -> unit_key, e.g.:
    // ("m", "*", "m") -> "m2"
    // ("m", "/", "s") -> "mps"
    let mut map: HashMap<(&str, &str, &str), &str> = HashMap::new();

    // First pass: collect all base units
    let mut all_units: HashMap<&str, &UnitDefinition> = HashMap::new();
    for (category, units) in definitions.categories.iter() {
        for (unit_key, unit) in units.iter() {
            if all_units.contains_key(unit_key.as_str()) {
                return Err(DefinitionError::DuplicatedUnit(
                    unit_key.clone(),
                    category.to_owned(),
                ));
            }
            all_units.insert(unit_key.as_str(), unit);
        }
    }

    // Second pass: process derived units
    for (_, units) in definitions.categories.iter() {
        for (unit, unit_def) in units.iter() {
            if let Some(derived_expr) = &unit_def.derived {
                // Parse simple expressions like "m * m" or "m / s"
                match derived_expr.split_whitespace().collect::<Vec<&str>>()[..] {
                    [left, op, right] => {
                        if all_units.contains_key(left) {
                            let key = (left, op, right);
                            match op {
                                "*" => {
                                    map.insert(key, unit);
                                    map.insert((unit, "/", left), right);
                                    map.insert((unit, "/", right), left);
                                }
                                "/" => {
                                    map.insert(key, unit);
                                    map.insert((left, "/", unit), right);
                                    map.insert((unit, "*", right), left);
                                }
                                _ => {
                                    return Err(DefinitionError::InvalidDerivedExpression(
                                        derived_expr.to_string(),
                                    ));
                                }
                            }
                        } else {
                            return Err(DefinitionError::UnitNotFound(
                                left.to_string(),
                                derived_expr.to_string(),
                            ));
                        }
                    }
                    _ => {
                        return Err(DefinitionError::InvalidDerivedExpression(
                            derived_expr.to_string(),
                        ));
                    }
                }
            }
        }
    }

    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_unit_translation_map() {
        let toml_str = r#"
[length]
m = { name = "meter", symbol = "m" }

[area]
m2 = { name = "square meter", symbol = "m²", derived = "m * m" }

[time]
s = { name = "second", symbol = "s" }

[speed]
mps = { name = "meters per second", symbol = "m/s", derived = "m / s" }
"#;
        let definitions: UnitDefinitions = toml::from_str(toml_str).unwrap();
        let map = construct_unit_translation_map(&definitions).unwrap();

        // Test m * m -> m² (area)
        assert_eq!(map.get(&("m", "*", "m")).unwrap(), &"m2");
        assert_eq!(map.get(&("m2", "/", "m")).unwrap(), &"m");

        // Test m / s -> m/s (speed)
        assert_eq!(map.get(&("m", "/", "s")).unwrap(), &"mps");
        assert_eq!(map.get(&("mps", "*", "s")).unwrap(), &"m");
        assert_eq!(map.get(&("m", "/", "mps")).unwrap(), &"s");
    }
}
