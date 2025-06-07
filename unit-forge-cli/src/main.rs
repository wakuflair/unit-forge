use std::fs;
use unit_forge_lib::UnitDefinitions;

fn main() {
    let content =
        fs::read_to_string("unit_definitions.toml").expect("Failed to read unit definitions file");

    let definitions: UnitDefinitions =
        toml::from_str(&content).expect("Failed to parse unit definitions");

    // Print out all categories and their units
    for (category, units) in &definitions.categories {
        println!("\nCategory: {}", category);
        for (symbol, unit) in units {
            print!("  {} ({})", symbol, unit.name);
            if let Some(factor) = &unit.factor {
                print!(" [factor: {}]", factor);
            }
            if let Some(derived) = &unit.from {
                print!(" [derived: {}]", derived);
            }
            println!();
        }
    }
}
