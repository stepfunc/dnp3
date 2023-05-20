Commercial library by [Step Function I/O](https://stepfunc.io/)

# DNP3

Rust implementation of DNP3 (IEEE 1815) with idiomatic bindings for C, C++, .NET, and Java.

# Features

- Subset Level 3 master and outstation components in a single library
- Written in safe Rust with idiomatic bindings for C, C++ .NET Core, and Java.
- Supports TCP, TLS,  and serial communication channels
- TLS is implemented using [rustls](https://github.com/rustls/rustls) not openssl.
- Automatic mapping between DNP3 and higher-level measurement types
- Built-in logging and protocol decoding
- Blazing fast (and secure) zero-copy / zero-allocation parsing of application data
- Fully asynchronous implementation scales to the OS limit.
- Runs on all platforms and operating systems supported by the [Tokio](https://tokio.rs/) runtime:
  - Official support for: Windows x64 and Linux x64, AArch64, ARMv7 and ARMv6
  - Unofficial support: MacOS, PowerPC, MIPS, FreeBSD, and others

# Cargo Features

Default features can be disabled at compile time:
* `tls` - Build the library with support for mutually authenticated TLS
* `serial` - Build the library with support for serial ports

Optional features that may be enabled at compile time:
* `serialize` - Add [serde](https://docs.rs/crate/serde/latest) de(serialization) support for public configuration types.

# License

This crate is made available under a non-commercial / non-production license.
Refer to [`LICENSE.txt`](https://raw.githubusercontent.com/stepfunc/dnp3/main/LICENSE.txt) for the terms
of this non-commercial license.

This software is publicly available, but is not "open source".
__You must purchase a commercial license to use this software for profit.__

Please inquire about commercial licensing on our website:

[https://stepfunc.io/contact/](https://stepfunc.io/contact/)

## Bindings

Bindings in C, C++, java, and .NET Core are available for this library. See the
[documentation](https://stepfunc.io/products/libraries/dnp3/) for more details.
