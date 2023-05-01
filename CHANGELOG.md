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
