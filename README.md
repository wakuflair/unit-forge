# Unit Forge

Unit Forge is a flexible, extensible command-line calculator for arithmetic with physical units. It allows you to define your own units and categories, perform calculations, and convert between units interactively.

## Features
- Arithmetic with units (e.g., `1 m + 2 cm`, `3 m * 4 m`)
- Custom unit definitions via TOML files
- Derived units and categories (e.g., area, volume, speed)
- Unit conversion (e.g., `1 m >> cm`)
- Variable assignment and reuse
- Error reporting for incompatible or unknown units

## Getting Started

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) (edition 2021 or later)

### Build
```powershell
cargo build --release
```

### Run
```powershell
cargo run --release -p unit-forge-cli
```

### Usage
- On launch, the CLI loads all `.ud` files from the `unit_definitions/` directory.
- Enter expressions such as:
  - `1 m + 2 cm`
  - `3 m * 4 m`
  - `1 m >> cm` (convert 1 meter to centimeters)
  - `x = 5 m` (assign variable)
  - `x + 2 m`
- Supported operators: `+`, `-`, `*`, `/`, `>>` (convert)
- Use parentheses for grouping: `(1 m + 2 m) * 3`

### Defining Units
Units are defined in TOML-like `.ud` files in the `unit_definitions/` directory. Example:
```toml
[length]
m = { name = "meter", symbol = "m" }
cm = { name = "centimeter", symbol = "cm", factor = 0.01 }

[area]
m2 = { name = "square meter", symbol = "m²", derived = "m * m" }
```

## Project Structure
- `unit-forge-lib/`: Core library for parsing, evaluating, and managing units
- `unit-forge-cli/`: Command-line interface
- `unit_definitions/`: Example and user-defined unit files

## Testing
```powershell
cargo test
```

## License
MIT
