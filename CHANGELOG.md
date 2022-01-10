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
