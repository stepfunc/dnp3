![Step Function I/O](./sfio_logo.png)

Commercial library by [Step Function I/O](https://stepfunc.io/)

# DNP3

Rust implementation of DNP3 (IEEE 1815) with idiomatic bindings for C, C++, .NET, and Java.

# Features

- Subset Level 3 master and outstation components in a single library
- Supports TCP and serial communication channels
- Support TLS through [rustls](https://github.com/rustls/rustls).
- Written in safe Rust with idiomatic bindings for C, C++ .NET Core, and Java.
- Blazing fast (and secure) zero-copy / zero-allocation parsing of application data
- Automatic mapping between DNP3 and higher-level measurement types
- Built-in logging and protocol decoding
- Share runtime resources with other libraries to implement extremely efficient gateways and translators
- Runs on all platforms and operating systems supported by the tokio runtime:
  - Official support for: Windows x64 and Linux x64, AArch64, ARMv7 and ARMv6
  - Non-official support: MacOS, PowerPC, MIPS, FreeBSD, and others

# License

This crate is made available under a non-commercial / non-production license.
Refer to [`LICENSE.txt`](./LICENSE.txt) for the terms of the non-commercial license.

This software is publicly available, but is not "open source".
__You must purchase a commercial license to use this software for profit.__

Please inquire about commercial licensing here:

[https://stepfunc.io/contact/](https://stepfunc.io/contact/)
