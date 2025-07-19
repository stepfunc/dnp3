#!/bin/bash

# DNP3 Code Generator Script
# This script runs the Scala 3 code generator to create Rust source files

set -e

# Change to the script's directory
cd "$(dirname "$0")"

echo "Running DNP3 code generator..."

# Clean and compile the project
mvn clean compile

# Run the code generator
mvn exec:java -Dexec.mainClass="dev.gridio.dnp3.codegen.Main"

echo "Code generation complete!"
echo "Generated files:"
echo "  - ../src/app/app_enums.rs"
echo "  - ../src/app/control_enums.rs"
echo "  - ../src/app/variations.rs"
echo "  - ../src/app/gen/all.rs"
echo "  - ../src/app/gen/conversion.rs"
echo "  - ../src/app/gen/count.rs"
echo "  - ../src/app/gen/prefixed.rs"
echo "  - ../src/app/gen/ranged.rs"