### 1.3.0-rc3 ###
* :star: Add master and outstation support for device attributes (group 0).
* :star: Add ability to specify TCP/TLS client local adapter and connect timeout. See [#254](https://github.com/stepfunc/dnp3/pull/254).
* :star: Add master support for receiving frozen analog inputs (groups 31 and 33). See [#256](https://github.com/stepfunc/dnp3/pull/256).
* :star: Add master and outstation support for analog inputs dead-bands (group 34). See [#257](https://github.com/stepfunc/dnp3/pull/257).
* :star: Add master API for sending freeze requests. Add freeze-at-time to outstation. See [#263](https://github.com/stepfunc/dnp3/pull/263).
* :star: Add a mechanism to the bindings to shut down the Runtime with a timeout. See [#271](https://github.com/stepfunc/dnp3/pull/271).
* :star: Add outstation APIs for tracking the lifetime of events. See [#273](https://github.com/stepfunc/dnp3/pull/273).
* :star: Add enable/disable methods to outstation instances. See [#278](https://github.com/stepfunc/dnp3/pull/278).
* :star: Add TCP/TLS client modes to the outstation API. See [#279](https://github.com/stepfunc/dnp3/pull/279).
* :bug: Fix incorrect encoding of octet-string events when adjacent events have different lengths. See [#269](https://github.com/stepfunc/dnp3/pull/270).
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
* Public API changes (affects both Rust and bindings):
  * User-controlled DB locking in callbacks. See [#189](https://github.com/stepfunc/dnp3/pull/189).
  * Multiple renamings to more closely follow the spec. See [#164](https://github.com/stepfunc/dnp3/pull/164) and others.
    Notably the following:
    * Binary -> BinaryInput
    * DoubleBitBinary -> DoubleBitBinaryInput
    * Analog -> AnalogInput
    * `TcpServer` -> `Server` (`OutstationServer` in bindings). Rename creation methods.
    * `ClassZeroConfig::octet_strings` -> `ClassZeroConfig::octet_string`.
  * Move response timeouts from `MasterChannelConfig` to `AssociationConfig`. See [#167](https://github.com/stepfunc/dnp3/pull/167).
  * Add `HeaderInfo::is_event` and `HeaderInfo::has_flags` fields. See [#143](https://github.com/stepfunc/dnp3/pull/143).
  * Add `AssociationInformation` to carry information about tasks success and failures.
    See [#182](https://github.com/stepfunc/dnp3/pull/182).
  * Add `AssociationHandle::read_with_handle()` to send a read request with a custom read handler.
    See [#178](https://github.com/stepfunc/dnp3/issues/178).
  * Add support for limited count qualifiers (`0x07` and `0x08`) for read requests.
    See [#179](https://github.com/stepfunc/dnp3/issues/179).
  * Re-order TLS channel arguments order.
  * `EventBufferConfig` is now part of the `OutstationConfig`.
  * Rename `UpdateOptions::initialized()` -> `UpdateOptions::no_event()`. Add `UpdateOptions::detect_event`.
* Rust-only changes:
  * Add feature flag for TLS support. See [#171](https://github.com/stepfunc/dnp3/pull/171).
  * `ReadHandler::begin()`, `ReadHandler::end()`, `ControlHandler::end()` and `Listener<T>::update()`
    now returns a `MaybeAsync` type to allow for asynchronous operations.
    See [#166](https://github.com/stepfunc/dnp3/pull/166) and [#183](https://github.com/stepfunc/dnp3/pull/183).
  * Remove `NullReadHandler`, `DefaultAssociationHandler`, `DefaultOutstationApplication` and `DefaultOutstationInformation` impls.
    All their methods have default implementations, so it is very easy to recreate them.
  * Remove `EventClasses::to_classes()` and `Classes::to_request()`.
  * Add `Classes::class123()` and `Classes::class0()`.
  * `OutstationApplication::write_absolute_time()` and `OutstationApplication::freeze_counter()` returns a `RequestError`.
  * `Bytes` was completely removed. `ReadHandler::handle_octet_string()` gets a `&'a [u8]` instead.
  * Rename `OctetStringError` -> `OctetStringLengthError`.
  * `OutstationConfig` timeouts use `Timeout`. Rename `RangeError` -> `TimeoutRangeError`.
  * Rename `AssociationHandler::get_system_time()` -> `Associationhandler::get_current_time()`.
  * Rename `EndpointAddress::from()` -> `EndpointAddress::try_new()`
  * `Association::check_link_status()` returns `Result<(), TaskError>`.
  * `Channel::remove_association()` returns `Result<(), Shutdown>` instead of `Result<(), AssociationError>`.
  * The `Cargo.toml` file is now commited in the repository for reproducibility.
  * Move to Rust 2021 edition.
* Bindings (C, C++, .NET and Java) changes:
  * Add C++ bindings. See [oo-bindgen#79](https://github.com/stepfunc/oo_bindgen/pull/79).
  * Windows 32-bit (x86) support. See [oo-bindgen#87](https://github.com/stepfunc/oo_bindgen/pull/87).
  * MacOS x64 builds included in the pre-built packages (**but not officially supported**).
    See [#168](https://github.com/stepfunc/dnp3/pull/168).
  * Other unsupported platforms can be built manually. Instructions were added in the docs.
    See [oo-bindgen#83](https://github.com/stepfunc/oo_bindgen/pull/83).
  * Better code documentation generation in all languages.
  * [C#] NuGet package now properly copies the DLL on .NET Framework using a MSBuild target files.
    See [#147](https://github.com/stepfunc/dnp3/issues/147).
  * [Java] Fix issue when loading the native library on a few platforms. See [#177](https://github.com/stepfunc/dnp3/issues/177).
  * [C#] and [Java] now have builder methods that can be chained. See [oo-bindgen#79](https://github.com/stepfunc/oo_bindgen/pull/79).
  * Octet string is now a byte iterator instead of an iterator of a struct. See [#162](https://github.com/stepfunc/dnp3/pull/162).
  * Rename `WriteTimeResult::InvalidValue` -> `WriteTimeResult::ParameterError`.
  * Add `Request::all_objects()`, `Request::one_byte_range()` and `Request::two_byte_range()`.
  * Add `ControlCode::from_op_type()` and `ControlCode::from_tcc_and_op_type()`.
  * Add `Group12Var1::from_code()`
  * Renamings:
    * `SerialPortSettings` -> `SerialSettings`
    * `Control` -> `ControlField`
    * `FreezeResult::Success` -> `FreezeResult::Ok`
    * `WriteTimeResult::InvalidValue` -> `WriteTimeResult::ParameterError`
  * General refactoring of the code generator.
* Miscellaneous:
  * Documentation is now built by the CI pipeline. See [#161](https://github.com/stepfunc/dnp3/pull/161).
  * Merge all examples in a single executable. See [#170](https://github.com/stepfunc/dnp3/pull/170).

### 0.10.0 ###
* :star: Add TLS support through `rustls`.
  See [#135](https://github.com/stepfunc/dnp3/pull/135).
* :bug: Fix issue where C# callbacks would be executed in Rust thread.
  See [oo-bindgen#5e95cf0f3989ad2d7b13eb8bba812e7e66805cea](https://github.com/stepfunc/oo_bindgen/commit/5e95cf0f3989ad2d7b13eb8bba812e7e66805cea).
* :wrench: C# and Java now checks that the native library has the same version
  as the generated code. See [oo-bindgen#75](https://github.com/stepfunc/oo_bindgen/pull/75).
* :bug: Fix issue where C# DLL was not properly copied when using .NET Framework.
  [oo-bindgen#75](https://github.com/stepfunc/oo_bindgen/pull/75).

### 0.9.2 ###
* :bug: Fix range scan issue where old values were reported.
  See [#148](https://github.com/stepfunc/dnp3/pull/148).
* :bug: Fix leak of tracing::Span in bindings.
  See [#139](https://github.com/stepfunc/dnp3/pull/139).
* :star: Add Linux AArch64 support in Java and .NET.
  See [#137](https://github.com/stepfunc/dnp3/pull/137).
* :star: Add support for serial ports on Windows.
  See [#134](https://github.com/stepfunc/dnp3/pull/134).

### 0.9.1 ###
* :star: C bindings now provides static libraries with the `dnp3_static` CMake target.
  See [#128](https://github.com/stepfunc/dnp3/pull/128).

### 0.9.0 ###
* :tada: First official release
