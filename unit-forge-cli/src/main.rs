use color_eyre::eyre::{Ok, Result};

fn main() {}

// fn main() -> Result<()> {
//     color_eyre::install()?;
//     let unit_definitions = parse_unit_definitions()?;
//     println!("Unit Definitions: {:#?}", unit_definitions);

//     let unit = UnitTable::new(&unit_definitions)?;
//     println!("Unit: {:#?}", unit);

//     Ok(())
// }

// fn parse_unit_definitions() -> Result<UnitDefinitions> {
//     let entries = std::fs::read_dir("unit_definitions")?;

//     let mut all_defs = UnitDefinitions::default();

//     for entry in entries {
//         let path = entry?.path();
//         if path.extension().and_then(|s| s.to_str()) == Some("ud") {
//             let content = std::fs::read_to_string(&path)?;
//             let defs: UnitDefinitions = toml::from_str(&content)?;
//             all_defs.categories.extend(defs.categories);
//         }
//     }

//     Ok(all_defs)
// }
