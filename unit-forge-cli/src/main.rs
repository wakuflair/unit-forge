use color_eyre::eyre::{Ok, Result};
use unit_forge_lib::UnitDefinitions;

fn main() -> Result<()> {
    color_eyre::install()?;
    let unit_definitions = parse_unit_definitions()?;
    println!("Unit Definitions: {:#?}", unit_definitions);

    let map = unit_forge_lib::construct_unit_translation_map(&unit_definitions)?;
    println!("Unit Translation Map: {:#?}", map);

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
