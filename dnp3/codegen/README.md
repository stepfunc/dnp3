# DNP3 Code Generator

This Scala 3 application generates Rust code for the DNP3 protocol implementation. It creates various enumeration types and parsing/serialization code based on the DNP3 protocol specification.

## Overview

The code generator creates the following Rust source files:

- `../src/app/app_enums.rs` - Application layer enumerations (FunctionCode, QualifierCode)
- `../src/app/control_enums.rs` - Control-related enumerations (CommandStatus, OpType, TripCloseCode)
- `../src/app/variations.rs` - Variation definitions and fixed-size parsing implementations
- `../src/app/gen/ranged.rs` - Range-based variations
- `../src/app/gen/all.rs` - All objects variations
- `../src/app/gen/count.rs` - Count-based variations
- `../src/app/gen/prefixed.rs` - Prefixed variations (with size prefix)
- `../src/app/gen/conversion.rs` - Type conversion implementations

## Requirements

- Java 8 or higher
- Maven 3.6 or higher
- Scala 3.3.1 (handled by Maven)

## Running the Code Generator

From the `dnp3/codegen` directory:

```bash
# Compile and run the generator
mvn compile exec:java

# Or if you need to clean first
mvn clean compile exec:java
```

## Architecture

The code generator is structured as follows:

- `Main.scala` - Entry point that orchestrates code generation
- `model/` - Data models representing DNP3 protocol elements
  - `EnumModel.scala` - Enumeration definitions
  - `Fields.scala` - Field type definitions
  - `ObjectGroup.scala` - DNP3 object group definitions
  - `Variation.scala` - Variation definitions
  - `groups/` - Individual group definitions (Group0-Group113)
  - `enums/protocol/` - Protocol enumeration definitions
- `render/` - Code rendering utilities
  - `Module.scala` - Base trait for code generation modules
  - `RenderUtils.scala` - Utility functions for code formatting
  - `modules/` - Individual code generation modules for different file types

## Development

The project uses Scala 3 with the following key features:
- Given instances for implicit conversions
- Extension methods for string formatting
- Pattern matching for code generation logic

To modify the code generator:
1. Update the relevant model definitions in `model/`
2. Modify or add rendering modules in `render/modules/`
3. Run the generator to produce new Rust code
4. Test the generated Rust code compiles correctly

## Testing

Run the test suite:

```bash
mvn test
```