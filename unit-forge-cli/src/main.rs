use color_eyre::eyre::{Ok, Result};
use unit_forge_lib::UnitDefinitions;

fn main() -> Result<()> {
    color_eyre::install()?;
    let unit_definitions = parse_unit_definitions()?;

    // Print out all categories and their units
    for (category, units) in &unit_definitions.categories {
        println!("\nCategory: {}", category);
        for (key, unit) in units {
            print!("  {} ({}, symbol: {})", key, unit.name, unit.symbol);
            if unit.factor != 1.0 {
                print!(", factor: {}", unit.factor);
            }
            if let Some(derived) = &unit.derived {
                print!(", derived: {}", derived);
            }
            println!();
        }
    }

    Ok(())
}

fn parse_unit_definitions() -> Result<UnitDefinitions> {
    let entries = std::fs::read_dir("unit_definitions")?;

    let mut all_defs = UnitDefinitions::default();

    for entry in entries {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("ud") {
            let content = std::fs::read_to_string(&path)?;
            let defs: UnitDefinitions = toml::from_str(&content)?;
            all_defs.categories.extend(defs.categories);
        }
    }

    Ok(all_defs)
}
