use std::collections::HashMap;

use super::*;

pub type UnitMapType<'a> = HashMap<(&'a str, &'a str, &'a str), &'a str>;
pub type BaseUnitMapType<'a> = HashMap<&'a str, (f64, &'a str)>;

#[derive(Debug)]
pub struct UnitTable<'a> {
    unit_definitions: &'a UnitDefinitions,
    derived_units_map: UnitMapType<'a>,
    base_units_map: BaseUnitMapType<'a>,
}

impl<'a> UnitTable<'a> {
    pub fn new(unit_definitions: &'a UnitDefinitions) -> Result<Self, DefinitionError> {
        let derived_units_map = construct_unit_translation_map(unit_definitions)?;
        let base_units_map = construct_base_units_map(unit_definitions)?;
        Ok(Self { unit_definitions, derived_units_map, base_units_map })
    }

    pub fn unit_definitions(&self) -> &'a UnitDefinitions {
        self.unit_definitions
    }

    pub fn derived_units_map(&self) -> &UnitMapType {
        &self.derived_units_map
    }

    pub fn base_units_map(&self) -> &BaseUnitMapType {
        &self.base_units_map
    }
}

fn construct_unit_translation_map(
    definitions: &UnitDefinitions,
) -> Result<UnitMapType, DefinitionError> {
    // (unit_key, op, unit_key) -> unit_key, e.g.:
    // ("m", "*", "m") -> "m2"
    // ("m", "/", "s") -> "mps"
    let mut map: UnitMapType = UnitMapType::new();

    // First pass: collect all units
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
    for (category, units) in definitions.categories.iter() {
        for (unit, unit_def) in units.iter() {
            if let Some(derived_expr) = &unit_def.derived {
                let parts: Vec<&str> = derived_expr.split_whitespace().collect();
                
                // Must have odd number of parts (alternating unit and operator)
                if parts.len() >= 3 && parts.len() % 2 == 1 {
                    let mut current_unit = parts[0];
                    let mut i = 1;
                    
                    // Validate first unit exists
                    if !all_units.contains_key(current_unit) {
                        return Err(DefinitionError::UnitNotFound(
                            current_unit.to_string(),
                            derived_expr.to_string(),
                            category.to_string(),
                        ));
                    }

                    while i < parts.len() - 1 {
                        let op = parts[i];
                        let next_unit = parts[i + 1];
                        
                        // Validate operator
                        if op != "*" && op != "/" {
                            return Err(DefinitionError::InvalidDerivedExpression(
                                derived_expr.to_string(),
                            ));
                        }
                        
                        // Validate next unit exists
                        if !all_units.contains_key(next_unit) {
                            return Err(DefinitionError::UnitNotFound(
                                next_unit.to_string(),
                                derived_expr.to_string(),
                                category.to_string(),
                            ));
                        }

                        // For intermediate operations, look up result in map if needed
                        let result_unit = if i == parts.len() - 2 {
                            // Last operation, result is our target unit
                            unit
                        } else {
                            // For intermediate operations (e.g., first m * m in m * m * m)
                            let key = (current_unit, op, next_unit);
                            if let Some(&result) = map.get(&key) {
                                result
                            } else {
                                return Err(DefinitionError::InvalidDerivedExpression(format!(
                                    "Cannot find intermediate unit for: {} {} {}",
                                    current_unit, op, next_unit
                                )));
                            }
                        };

                        // Add mappings for this operation
                        if op == "*" {
                            map.insert((current_unit, "*", next_unit), result_unit);
                            map.insert((next_unit, "*", current_unit), result_unit);
                            map.insert((result_unit, "/", current_unit), next_unit);
                            map.insert((result_unit, "/", next_unit), current_unit);
                        } else { // op == "/"
                            map.insert((current_unit, "/", next_unit), result_unit);
                            map.insert((current_unit, "/", result_unit), next_unit);
                            map.insert((result_unit, "*", next_unit), current_unit);
                        }
                        
                        // Set up for next iteration
                        current_unit = result_unit;
                        i += 2;
                    }
                } else {
                    return Err(DefinitionError::InvalidDerivedExpression(
                        derived_expr.to_string(),
                    ));
                }
            }
        }
    }

    Ok(map)
}

fn construct_base_units_map(
    definitions: &UnitDefinitions,
) -> Result<BaseUnitMapType, DefinitionError> {
    let mut base_units_map: BaseUnitMapType = BaseUnitMapType::new();

    for (category, units) in definitions.categories.iter() {
        let base_unit = units.first().ok_or_else(|| {
            DefinitionError::NoUnitDefined(category.to_string())
        })?.0;
        for (unit_key, unit_def) in units.iter() {
            base_units_map.insert(unit_key, (unit_def.factor, base_unit));
        }
    }

    Ok(base_units_map)
}


#[cfg(test)]
mod tests {
    use crate::{DefinitionError, UnitDefinitions};

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

    #[test]
    fn test_multiple_operators() {
        let toml_str = r#"
[length]
m = { name = "meter", symbol = "m" }

[area]
m2 = { name = "square meter", symbol = "m²", derived = "m * m" }

[volume]
m3 = { name = "cubic meter", symbol = "m³", derived = "m * m * m" }
"#;
        let definitions: UnitDefinitions = toml::from_str(toml_str).unwrap();
        let map = construct_unit_translation_map(&definitions).unwrap();

        // Test basic area operations
        assert_eq!(map.get(&("m", "*", "m")).unwrap(), &"m2");
        assert_eq!(map.get(&("m2", "/", "m")).unwrap(), &"m");

        // Test volume operations
        assert_eq!(map.get(&("m2", "*", "m")).unwrap(), &"m3");
        assert_eq!(map.get(&("m3", "/", "m")).unwrap(), &"m2");
        assert_eq!(map.get(&("m3", "/", "m2")).unwrap(), &"m");
    }

    #[test]
    fn test_duplicate_unit_error() {
        let toml_str = r#"
[length]
m = { name = "meter", symbol = "m" }

[area]
m = { name = "another meter", symbol = "m" }
"#;
        let definitions: UnitDefinitions = toml::from_str(toml_str).unwrap();
        let err = construct_unit_translation_map(&definitions).unwrap_err();
        println!("Error: {}", err);
        assert!(matches!(err, DefinitionError::DuplicatedUnit(unit, category) 
            if unit == "m" && category == "area"));
    }

    #[test]
    fn test_invalid_derived_expression() {
        let toml_str = r#"
[length]
m = { name = "meter", symbol = "m" }

[area]
m2 = { name = "square meter", symbol = "m²", derived = "m ** m" }
"#;
        let definitions: UnitDefinitions = toml::from_str(toml_str).unwrap();
        let err = construct_unit_translation_map(&definitions).unwrap_err();
        assert!(matches!(err, DefinitionError::InvalidDerivedExpression(expr) 
            if expr == "m ** m"));

        // Test invalid operator
        let toml_str = r#"
[length]
m = { name = "meter", symbol = "m" }

[area]
m2 = { name = "square meter", symbol = "m²", derived = "m + m" }
"#;
        let definitions: UnitDefinitions = toml::from_str(toml_str).unwrap();
        let err = construct_unit_translation_map(&definitions).unwrap_err();
        assert!(matches!(err, DefinitionError::InvalidDerivedExpression(expr) 
            if expr == "m + m"));
    }

    #[test]
    fn test_undefined_unit_in_derived() {
        let toml_str = r#"
[area]
m2 = { name = "square meter", symbol = "m²", derived = "x * x" }
"#;
        let definitions: UnitDefinitions = toml::from_str(toml_str).unwrap();
        let err = construct_unit_translation_map(&definitions).unwrap_err();
        assert!(matches!(err, DefinitionError::UnitNotFound(unit, expr, category) 
            if unit == "x" && expr == "x * x" && category == "area"));
    }
}
