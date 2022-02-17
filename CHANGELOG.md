### 1.0.0 ###
* Merge all examples in a single executable. See [#170](https://github.com/stepfunc/dnp3/pull/170).
* Add feature flag for TLS support. See [#171](https://github.com/stepfunc/dnp3/pull/171).
* Move response timeouts from `MasterChannelConfig` to `AssociationConfig`. See [#167](https://github.com/stepfunc/dnp3/pull/167).
* `ReadHandler::begin()`, `ReadHandler::end()` and `ControlHandler::end()` now returns a
  `MaybeAsync` type to allow for asynchronous operations. See [#166](https://github.com/stepfunc/dnp3/pull/166).
* MacOS x64 builds are now included in the pre-built packages (**but not officially supported**).
  See [#168](https://github.com/stepfunc/dnp3/pull/168).
* Documentation is now built by the CI pipeline. See [#161](https://github.com/stepfunc/dnp3/pull/161).
* Octet string is now a byte iterator instead of an iterator of a struct. See [#162](https://github.com/stepfunc/dnp3/pull/162).
* Add `HeaderInfo::is_event` and `HeaderInfo::has_flags` fields. See [#143](https://github.com/stepfunc/dnp3/pull/143).
* Renamings to more closely follow the spec. See [#164](https://github.com/stepfunc/dnp3/pull/164).
  * `Binary` -> `BinaryInput`
  * `BinaryConfig`-> `BinaryInputConfig`
  * `StaticBinaryVariation` -> `StaticBinaryInputVariation`
  * `EventBinaryVariation` -> `EventBinaryInputVariation`
  * `ReadHandler::handle_binary()` -> `ReadHandler::handle_binary_input()`
  * `Database::add_binary()` -> `Database::add_binary_input()`
  * `Database::update_binary()` -> `Database::update_binary_input()`
  * `Database::remove_binary()` -> `Database::remove_binary_input()`
  * `Database::get_binary()` -> `Database::get_binary_input()`
  * `DoubleBitBinary` -> `DoubleBitBinaryInput`
  * `DoubleBitBinaryConfig`-> `DoubleBitBinaryInputConfig`
  * `StaticDoubleBitBinaryVariation` -> `StaticDoubleBitBinaryInputVariation`
  * `EventDoubleBitBinaryVariation` -> `EventDoubleBitBinaryInputVariation`
  * `ReadHandler::handle_double_bit_binary()` -> `ReadHandler::handle_double_bit_binary_input()`
  * `Database::add_double_bit_binary()` -> `Database::add_double_bit_binary_input()`
  * `Database::update_double_bit_binary()` -> `Database::update_double_bit_binary_input()`
  * `Database::remove_double_bit_binary()` -> `Database::remove_double_bit_binary_input()`
  * `Database::get_double_bit_binary()` -> `Database::get_double_bit_binary_input()`
  * `Analog` -> `AnalogInput`
  * `AnalogConfig`-> `AnalogInputConfig`
  * `StaticAnalogVariation` -> `StaticAnalogInputVariation`
  * `EventAnalogVariation` -> `EventAnalogInputVariation`
  * `ReadHandler::handle_analog()` -> `ReadHandler::handle_analog_input()`
  * `Database::add_analog()` -> `Database::add_analog_input()`
  * `Database::update_analog()` -> `Database::update_analog_input()`
  * `Database::remove_analog()` -> `Database::remove_analog_input()`
  * `Database::get_analog()` -> `Database::get_analog_input()`

* C# and Java now has builder methods that can be chained. See [oo-bindgen#79](https://github.com/stepfunc/oo_bindgen/pull/79).
* Add C++ bindings. See [oo-bindgen#79](https://github.com/stepfunc/oo_bindgen/pull/79).
* 

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
