use std::io::{Write, stdout};

use color_eyre::eyre::Result;
use unit_forge_lib::{Interpretor, UnitDefinitions};

fn main() -> Result<()> {
    color_eyre::install()?;
    let unit_definitions = parse_unit_definitions()?;
    let mut interpretor = Interpretor::new(&unit_definitions)?;

    // read expressions from stdin
    loop {
        print!("> ");
        stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        match interpretor.execute_command(&input) {
            Ok(val) => {
                println!("{} {}", val.0, val.1);
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        }
    }
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
