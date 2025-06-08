use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct UnitDefinition {
    pub name: String,
    pub symbol: String,
    #[serde(default = "default_factor")]
    pub factor: f64,
    #[serde(default)]
    pub derived: Option<String>,
}

fn default_factor() -> f64 {
    1.0
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct UnitDefinitions {
    #[serde(flatten)]
    pub categories: HashMap<String, HashMap<String, UnitDefinition>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_area_units_deserialize() {
        let toml_str = r#"
[area]
m2 = { name = "square meter", symbol = "m²", derived = "m * m" }
cm2 = { name = "square center meter", symbol = "cm²", factor = "10000" }
        "#;

        let definitions: UnitDefinitions = toml::from_str(toml_str).unwrap();
        let area_units = definitions.categories.get("area").unwrap();

        let m2 = area_units.get("m2").unwrap();
        assert_eq!(m2.name, "square meter");
        assert_eq!(m2.symbol, "m²");
        assert_eq!(m2.factor, 1.0);
        assert_eq!(m2.derived.as_ref().unwrap(), "m * m");

        let cm2 = area_units.get("cm2").unwrap();
        assert_eq!(cm2.name, "square center meter");
        assert_eq!(cm2.symbol, "cm²");
        assert_eq!(cm2.factor, 10000.0);
        assert_eq!(cm2.derived, None);
    }
}
