#!/bin/bash
# Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
# No use to design, develop, specify, test, validate, or improve Other Implementations,
# subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
# Text and data mining rights, including for AI training, are reserved under
# Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

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
echo "  - ../src/app/gen/read.rs"
echo "  - ../src/master/extract/generated.rs"
echo "  - ../src/outstation/database/variations.rs"
echo "  - ../src/outstation/database/read/target.rs"
echo "  - ../../ffi/dnp3-schema/src/variation.rs"
echo "  - ../../ffi/dnp3-ffi/src/variation.rs"
echo "  - ../../ffi/dnp3-schema/src/database/variation.rs"
echo "  - ../../ffi/dnp3-ffi/src/outstation/database/variation.rs"