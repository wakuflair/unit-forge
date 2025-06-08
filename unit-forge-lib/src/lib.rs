use std::collections::HashMap;

mod units;
pub use units::*;

/// Constructs a map that links unit combinations to their derived units.
/// For example, if m² is derived from "m * m", the map will contain:
/// {
///     "m": {
///         "m": UnitDefinition of m² // When m is combined with m
///     }
/// }
pub fn construct_unit_translation_map(
    definitions: &UnitDefinitions,
) -> HashMap<String, HashMap<String, (String, UnitDefinition)>> {
    let mut translation_map: HashMap<String, HashMap<String, (String, UnitDefinition)>> =
        HashMap::new();

    // First pass: collect all base unit symbols for validation
    let mut known_symbols: HashMap<&str, &str> = HashMap::new();
    for units in definitions.categories.values() {
        for (key, unit) in units.iter() {
            if unit.derived.is_none() {
                known_symbols.insert(&unit.symbol, key);
            }
        }
    }

    // Second pass: process derived units
    for (category, units) in &definitions.categories {
        for (unit_key, unit_def) in units {
            if let Some(derived_expr) = &unit_def.derived {
                // Parse simple expressions like "m * m" or "m / s"
                let parts: Vec<&str> = derived_expr.split_whitespace().collect();
                if parts.len() == 3 && (parts[1] == "*" || parts[1] == "/") {
                    let first_unit = parts[0].trim();
                    let second_unit = parts[2].trim();
                    let operation = parts[1];

                    // Validate that the referenced units exist
                    if let (Some(first_key), Some(second_key)) = (
                        known_symbols.get(first_unit),
                        known_symbols.get(second_unit),
                    ) {
                        // Store the relationship with category info
                        translation_map
                            .entry(first_unit.to_string())
                            .or_default()
                            .insert(
                                second_unit.to_string(),
                                (category.clone(), unit_def.clone()),
                            );

                        // For multiplication, store the reverse relationship too (since a*b = b*a)
                        if operation == "*" {
                            translation_map
                                .entry(second_unit.to_string())
                                .or_default()
                                .insert(
                                    first_unit.to_string(),
                                    (category.clone(), unit_def.clone()),
                                );
                        }
                    }
                }
            }
        }
    }

    translation_map
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
        let translation_map = construct_unit_translation_map(&definitions);

        // Test m * m -> m² (area)
        let m_translations = translation_map.get("m").unwrap();
        let (category, m2_unit) = m_translations.get("m").unwrap();
        assert_eq!(category, "area");
        assert_eq!(m2_unit.symbol, "m²");

        // Test m / s -> m/s (speed)
        let m_translations = translation_map.get("m").unwrap();
        let (category, mps_unit) = m_translations.get("s").unwrap();
        assert_eq!(category, "speed");
        assert_eq!(mps_unit.symbol, "m/s");
    }
}
