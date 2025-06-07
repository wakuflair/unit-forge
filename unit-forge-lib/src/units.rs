use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnitDefinition {
    pub name: String,
    pub symbol: String,
    #[serde(default)]
    pub factor: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
m2 = { name = "square meter", symbol = "m²", from = "m * m" }
cm2 = { name = "square center meter", symbol = "cm²", factor = "10000" }
        "#;

        let definitions: UnitDefinitions = toml::from_str(toml_str).unwrap();
        let area_units = definitions.categories.get("area").unwrap();

        let m2 = area_units.get("m2").unwrap();
        assert_eq!(m2.name, "square meter");
        assert_eq!(m2.symbol, "m²");
        assert_eq!(m2.from.as_ref().unwrap(), "m * m");

        let cm2 = area_units.get("cm2").unwrap();
        assert_eq!(cm2.name, "square center meter");
        assert_eq!(cm2.symbol, "cm²");
        assert_eq!(cm2.factor.as_ref().unwrap(), "10000");
        assert_eq!(cm2.from, None);
    }
}
