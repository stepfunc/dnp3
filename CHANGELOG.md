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
