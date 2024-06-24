
### 1.6.0 ###
* :star: Add master station support for writing files to the outstation. See [#338](https://github.com/stepfunc/dnp3/pull/338).
* :star: Add master station support for command events (groups 13 and 43). See [#332](https://github.com/stepfunc/dnp3/pull/332).
* :star: Add master station support for group 102. See [#335](https://github.com/stepfunc/dnp3/pull/335).
* :star: Add master and outstation support for UDP. See [#353](https://github.com/stepfunc/dnp3/pull/353).
* :star: Add master station support for acting as a TCP server. See [#358](https://github.com/stepfunc/dnp3/pull/358).
* :star: Add ability to update flags and timestamp without knowing the current value. See [#365](https://github.com/stepfunc/dnp3/pull/365).
* :star: Obtain TCP port from server in Rust API. [#331](https://github.com/stepfunc/dnp3/pull/331).

### 1.5.2 ###
* :bug: Fix bug where the outstation would sleep before all unsolicited data was transmitted. See [#341](https://github.com/stepfunc/dnp3/pull/341).

### 1.5.1 ###
* :wrench: Update to oo-bindgen 0.8.6 to improve Java native library loading. See [oo-bindgen #124](https://github.com/stepfunc/oo_bindgen/pull/124).
* :bell: **This release only affects the Java distribution**. It is equivalent to 1.5.0 for other distributions.

### 1.5.0 ###
* :star: Add configuration option that allows outstation to respond to any master. See [#316](https://github.com/stepfunc/dnp3/pull/316).
* :star: Add optional [serde](https://crates.io/crates/serde) support for public config types in Rust. See [#303](https://github.com/stepfunc/dnp3/pull/303).
* :wrench: Fix cmake download when building C/C++ examples. See [#307](https://github.com/stepfunc/dnp3/pull/307).
* :book: Fix FFI docs for g12v1 and remove g12v0 and g41v0. See [#308](https://github.com/stepfunc/dnp3/pull/308)
* :bug: Fix master task scheduling CPU thrashing under certain conditions. See [#312](https://github.com/stepfunc/dnp3/pull/312).
* :bug: Fix bug where `AssociationInformation::task_fail` was not properly being called for some tasks. See [#313](https://github.com/stepfunc/dnp3/pull/313).
* :bug: Update rx509 to 0.2.1 to fix ASN.1 GeneralizedTime parsing. See [RASN #2](https://github.com/stepfunc/rasn/pull/2).

### 1.4.1 ###
* :bug: Bump rustls to 0.21.1 to resolve [#300](https://github.com/stepfunc/dnp3/issues/300).

### 1.4.0 ###
* :wrench: Update to rustls 0.21.0 which allows peer names with IP addresses in the SAN extension.
* :wrench: Move common TLS configuration to its own crate shared with our Modbus library.
* :star: PEM parser now supports extracting PKCS#1 private keys, i.e. PEM files with `BEGIN RSA PRIVATE KEY`.
* :star: X.509 name verification may now be disabled in the TLS client and server configurations.
* :book: Documentation improvements in the bindings via [oo-bindgen 0.8.3](https://github.com/stepfunc/oo_bindgen/blob/main/CHANGELOG.md).

### 1.3.0 ###
* :star: Add master and outstation support for device attributes (group 0).
* :star: Add master support for reading files and directories. See [#281](https://github.com/stepfunc/dnp3/pull/281).
* :star: Add ability to specify TCP/TLS client local adapter and connect timeout. See [#254](https://github.com/stepfunc/dnp3/pull/254).
* :star: Add master support for receiving frozen analog inputs (groups 31 and 33). See [#256](https://github.com/stepfunc/dnp3/pull/256).
* :star: Add master and outstation support for analog inputs dead-bands (group 34). See [#257](https://github.com/stepfunc/dnp3/pull/257).
* :star: Add master API for sending freeze requests. Add freeze-at-time to outstation. See [#263](https://github.com/stepfunc/dnp3/pull/263).
* :star: Add a mechanism to the bindings to shut down the Runtime with a timeout. See [#271](https://github.com/stepfunc/dnp3/pull/271).
* :star: Add outstation APIs for tracking the lifetime of events. See [#273](https://github.com/stepfunc/dnp3/pull/273).
* :star: Add enable/disable methods to outstation instances. See [#278](https://github.com/stepfunc/dnp3/pull/278).
* :star: Add TCP/TLS client modes to the outstation API. See [#279](https://github.com/stepfunc/dnp3/pull/279).
* :bug: Fix incorrect encoding of octet-string events when adjacent events have different lengths. See [#269](https://github.com/stepfunc/dnp3/pull/270).
* :bug: Fix bug where the outstation would respond to a master other than the one configured. See [#284](https://github.com/stepfunc/dnp3/pull/284).
* :book: Various FFI documentation improvements. See [#250](https://github.com/stepfunc/dnp3/pull/250).

### 1.2.0 ###
* :star: Enable TCP_NODELAY by default. See [#218](https://github.com/stepfunc/dnp3/pull/218).
* :star: Enable full link-time optimization (LTO) in release builds. See [#223](https://github.com/stepfunc/dnp3/pull/223).
* :star: Add support for 3 MUSL Linux targets to C/C++ and .NET. See [#228](https://github.com/stepfunc/dnp3/pull/228).
* :star: Use only dependencies from crates.io allowing first release there. See [#227](https://github.com/stepfunc/dnp3/pull/227).
* :star: Internal refactoring to promote code reuse with Rodbus. See: [#220](https://github.com/stepfunc/dnp3/pull/220), [#221](https://github.com/stepfunc/dnp3/pull/221), [#222](https://github.com/stepfunc/dnp3/pull/222).

### 1.1.0 ###
* :star: TCP/TLS server can now filter IPv4 addresses with wildcards. See [#208](https://github.com/stepfunc/dnp3/pull/208).
* :star: Rust crate and FFI/JNI libraries can now be compiled without TLS and/or serial support. See [#212](https://github.com/stepfunc/dnp3/pull/212).
* :star: Add a way to spawn a serial outstation that is tolerant to the port being removed from the OS. See [#215](https://github.com/stepfunc/dnp3/pull/215).
* :star: Produce enhanced third-party license reports for FFI and JNI bindings.

### 1.0.1 ###
* :bug: Fix panic when creating serial outstation [#203](https://github.com/stepfunc/dnp3/pull/203).
* :star: Log when OVERFLOW or EVENT classes available IIN bits causes a poll to be scheduled. [#25](https://github.com/stepfunc/dnp3/pull/205).
* :star: Build all Linux FFI/JNI artifacts using rust-cross [#197](https://github.com/stepfunc/dnp3/pull/197).
* :star: Newer rust-cross version produces more portable Linux libraries (older GLIBC).

### 1.0.0 ###
* First release with stable API
