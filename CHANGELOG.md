
### 1.8.0-M2 ###
* :bell: **License update.** The license now prohibits using the library to build or test other implementations of the protocol, including by mining it with AI tools or using it as an oracle, and reserves AI training. See [#712](https://github.com/stepfunc/dnp3/pull/712).
* :bell: **The minimum supported Rust version is now 1.88.** See [#502](https://github.com/stepfunc/dnp3/pull/502).
* :star: The outstation processes ASSIGN_CLASS (function code 22), which subset level 3 requires, by default. `OutstationApplication::support_assign_class` turns it off, and new callbacks report each accepted request so the application can persist it. See [#856](https://github.com/stepfunc/dnp3/pull/856).
* :shield: Time out TLS handshakes after 5 seconds, in the outstation server and in TLS clients, so a peer that stalls a handshake can't hold up other connections or retries. See [#532](https://github.com/stepfunc/dnp3/pull/532) and [#533](https://github.com/stepfunc/dnp3/pull/533).
* :shield: Keep TCP servers accepting connections after an accept error, such as running out of file descriptors. See [#638](https://github.com/stepfunc/dnp3/pull/638).
* :shield: Keep UDP masters and outstations running after an empty datagram, a datagram whose source can't be answered, a datagram larger than the read buffer on Windows, or a peer that isn't listening yet. See [#544](https://github.com/stepfunc/dnp3/pull/544), [#637](https://github.com/stepfunc/dnp3/pull/637), [#699](https://github.com/stepfunc/dnp3/pull/699), [#510](https://github.com/stepfunc/dnp3/pull/510) and [#524](https://github.com/stepfunc/dnp3/pull/524).
* :shield: Budget the work an outstation spends on a READ or an ASSIGN_CLASS, scaled to the size of its database, so that a request repeating headers that name every point or event is answered with IIN2.2 instead of consuming the CPU. See [#855](https://github.com/stepfunc/dnp3/pull/855).
* :shield: Leave file AUTHENTICATE passwords and auth keys out of decode logging and `Debug` output. See [#520](https://github.com/stepfunc/dnp3/pull/520).
* :shield: Update `tokio` to 1.54.1 to resolve [RUSTSEC-2026-0386](https://rustsec.org/advisories/RUSTSEC-2026-0386.html), in which `AsyncFd`'s safe constructors could register a file descriptor the caller doesn't own and cause a use-after-free. dnp3 doesn't construct `AsyncFd`; its serial support reaches it only through `tokio-serial`, which registers the serial port it owns for the port's lifetime, so dnp3 is not exposed. The update only matters for the prebuilt binary distributions of the bindings (C/C++, .NET, Java). See [#771](https://github.com/stepfunc/dnp3/pull/771).
* :shield: Update `rustls-webpki` to 0.103.16 to resolve [RUSTSEC-2026-0382](https://rustsec.org/advisories/RUSTSEC-2026-0382), in which certificate paths through distrusted emSign intermediates could be accepted. dnp3 trusts only the CA file it is configured with, so this requires `CertificateMode::AuthorityBased` with a public emSign root in that file; `CertificateMode::SelfSigned` is not affected. The update only matters for the prebuilt binary distributions of the bindings (C/C++, .NET, Java). See [#505](https://github.com/stepfunc/dnp3/pull/505).
* :bug: Fix the outstation losing the events of an unsolicited response that was never confirmed. See [#546](https://github.com/stepfunc/dnp3/pull/546).
* :bug: Fix an outstation READ including events of a class it didn't ask for. See [#634](https://github.com/stepfunc/dnp3/pull/634).
* :bell: Outstations answer a read of a point type that has no points with IIN2.1, and a range read that includes missing points with IIN2.2 while still returning the points that exist, as the standard requires. Masters that poll types or ranges the outstation doesn't have will see these errors where they saw none before. See [#487](https://github.com/stepfunc/dnp3/pull/487).
* :bell: Outstations report a counter, frozen counter or analog input whose flags are anything but ONLINE in a variation with flags, even when the master asked for one without, as the standard requires, so the master sees the point's quality. A response can now mix the requested variation with its flagged counterpart, for example right after startup, when every point carries RESTART. See [#484](https://github.com/stepfunc/dnp3/pull/484).
* :bell: Binary output status points default to g10v2 instead of g10v1, in Rust and in the bindings. See [#853](https://github.com/stepfunc/dnp3/pull/853).
* :bell: Outstations report events recorded without time in variations without time, and binary and double-bit input events recorded while unsynchronized with relative time. See [#744](https://github.com/stepfunc/dnp3/pull/744) and [#646](https://github.com/stepfunc/dnp3/pull/646).
* :bell: Analog values: adding a point with a NaN or negative deadband fails, NaN in an integer variation sets OVER_RANGE, a change between NaN and a number creates an event, a negative deadband written by a master is stored as 0, and a NaN or infinite analog output command is answered with NOT_SUPPORTED. See [#551](https://github.com/stepfunc/dnp3/pull/551), [#649](https://github.com/stepfunc/dnp3/pull/649), [#650](https://github.com/stepfunc/dnp3/pull/650), [#651](https://github.com/stepfunc/dnp3/pull/651) and [#652](https://github.com/stepfunc/dnp3/pull/652).
* :bell: The master processes responses that set IIN2 error bits instead of discarding them, acts on the IIN1 bits of the ones it rejects, and no longer confirms unsolicited responses it can't parse. See [#536](https://github.com/stepfunc/dnp3/pull/536), [#554](https://github.com/stepfunc/dnp3/pull/554) and [#598](https://github.com/stepfunc/dnp3/pull/598).
* :bell: The master confirms the response to any request when the outstation asks for a confirmation. See [#689](https://github.com/stepfunc/dnp3/pull/689).
* :bell: The master's automatic time sync runs before the integrity scan at startup and after a restart. See [#708](https://github.com/stepfunc/dnp3/pull/708).
* :bell: File and directory reads that end early send one CLOSE, and fail with the cause. See [#621](https://github.com/stepfunc/dnp3/pull/621) and [#626](https://github.com/stepfunc/dnp3/pull/626).
* :bell: Tasks whose response objects can't be parsed fail with `MalformedResponse`. See [#696](https://github.com/stepfunc/dnp3/pull/696) and [#702](https://github.com/stepfunc/dnp3/pull/702).
* :bell: `RetryStrategy` raises a delay of zero to 1 ms and swaps a reversed minimum and maximum. See [#570](https://github.com/stepfunc/dnp3/pull/570).
* :bell: Defining a string device attribute longer than 255 bytes fails with the new `AttrDefError::BadValue`. See [#567](https://github.com/stepfunc/dnp3/pull/567).
* :bell: Deprecate `WriteError::IinError`, `TimeSyncError::IinError`, `TimeSyncError::SystemTimeNotUnix`, `CommandResponseError::Request` and `ObjectParseError::UnsupportedQualifierCode`, which are never returned. They will be removed in 2.0. See [#556](https://github.com/stepfunc/dnp3/pull/556).
* :bell: Expected traffic, such as frames for other outstations and repeated segments, is logged at debug instead of warn. See [#685](https://github.com/stepfunc/dnp3/pull/685).
* :star: The bindings' `ReadHandler` receives analog input deadbands (group 34). See [#854](https://github.com/stepfunc/dnp3/pull/854).
* :star: Every public type implements `Debug`, `dnp3::link` exports its error types, and master task errors gain `unsupported_response`, `unsupported_or_bad_response` and `SystemTimeNotAvailable`. See [#519](https://github.com/stepfunc/dnp3/pull/519), [#539](https://github.com/stepfunc/dnp3/pull/539), [#653](https://github.com/stepfunc/dnp3/pull/653) and [#753](https://github.com/stepfunc/dnp3/pull/753).
* :bell: Many small outstation conformance corrections: the IIN2 bits for unsupported objects and qualifiers, discarding invalid request fragments, answering repeated and unparsable requests, the unsolicited confirm wait, select-before-operate, and analog input deadband qualifiers. See [#522](https://github.com/stepfunc/dnp3/pull/522), [#541](https://github.com/stepfunc/dnp3/pull/541), [#633](https://github.com/stepfunc/dnp3/pull/633), [#529](https://github.com/stepfunc/dnp3/pull/529), [#525](https://github.com/stepfunc/dnp3/pull/525), [#560](https://github.com/stepfunc/dnp3/pull/560), [#561](https://github.com/stepfunc/dnp3/pull/561), [#691](https://github.com/stepfunc/dnp3/pull/691), [#625](https://github.com/stepfunc/dnp3/pull/625), [#573](https://github.com/stepfunc/dnp3/pull/573), [#749](https://github.com/stepfunc/dnp3/pull/749), [#501](https://github.com/stepfunc/dnp3/pull/501), [#530](https://github.com/stepfunc/dnp3/pull/530) and [#648](https://github.com/stepfunc/dnp3/pull/648).
* :bug: Other outstation fixes. See [#852](https://github.com/stepfunc/dnp3/pull/852), [#504](https://github.com/stepfunc/dnp3/pull/504), [#513](https://github.com/stepfunc/dnp3/pull/513), [#619](https://github.com/stepfunc/dnp3/pull/619), [#644](https://github.com/stepfunc/dnp3/pull/644), [#697](https://github.com/stepfunc/dnp3/pull/697) and [#545](https://github.com/stepfunc/dnp3/pull/545).
* :bug: Master fixes in file transfer, device attributes, commands, channels shared by several outstations, automatic tasks and keep-alives, and parsing. See [#547](https://github.com/stepfunc/dnp3/pull/547), [#631](https://github.com/stepfunc/dnp3/pull/631), [#548](https://github.com/stepfunc/dnp3/pull/548), [#517](https://github.com/stepfunc/dnp3/pull/517), [#728](https://github.com/stepfunc/dnp3/pull/728), [#500](https://github.com/stepfunc/dnp3/pull/500), [#565](https://github.com/stepfunc/dnp3/pull/565), [#613](https://github.com/stepfunc/dnp3/pull/613), [#636](https://github.com/stepfunc/dnp3/pull/636), [#553](https://github.com/stepfunc/dnp3/pull/553), [#614](https://github.com/stepfunc/dnp3/pull/614), [#620](https://github.com/stepfunc/dnp3/pull/620), [#552](https://github.com/stepfunc/dnp3/pull/552), [#632](https://github.com/stepfunc/dnp3/pull/632), [#647](https://github.com/stepfunc/dnp3/pull/647), [#708](https://github.com/stepfunc/dnp3/pull/708), [#791](https://github.com/stepfunc/dnp3/pull/791) and [#593](https://github.com/stepfunc/dnp3/pull/593).
* :bug: Link and transport layer fixes. See [#508](https://github.com/stepfunc/dnp3/pull/508), [#586](https://github.com/stepfunc/dnp3/pull/586), [#550](https://github.com/stepfunc/dnp3/pull/550), [#658](https://github.com/stepfunc/dnp3/pull/658), [#635](https://github.com/stepfunc/dnp3/pull/635) and [#645](https://github.com/stepfunc/dnp3/pull/645).
* :wrench: The prebuilt libraries for the C/C++, .NET and Java bindings are about 20% smaller and faster. See [#798](https://github.com/stepfunc/dnp3/pull/798) and [#799](https://github.com/stepfunc/dnp3/pull/799).
* :book: Consistent variation descriptions in logging and the bindings' docs, and a guide page on the master's automatic tasks and scheduling. See [#781](https://github.com/stepfunc/dnp3/pull/781), [#787](https://github.com/stepfunc/dnp3/pull/787) and [#700](https://github.com/stepfunc/dnp3/pull/700).

### 1.8.0-M1 ###
* :bell: **The minimum supported Rust version is now 1.85**, which the updated dependencies require. CI now checks it.
* :star: Add master support for `DIRECT_OPERATE_NO_ACK` (function code 0x06) in Rust and all bindings. See [#466](https://github.com/stepfunc/dnp3/pull/466).
* :bug: Fix a double free in the bindings when an outstation server fails to bind, for example because the port is already in use. See [#469](https://github.com/stepfunc/dnp3/pull/469).
* :bug: Flush TLS writes so the last frame isn't left buffered when the socket is under backpressure. Affects masters and outstations. See [#462](https://github.com/stepfunc/dnp3/pull/462).
### 1.7.0 ###
* :star: Add outstation `ConnectionManager` for fine-grained control of TCP and TLS client connections, including dynamic endpoint selection, custom retry logic, and per-connection master address override. See [#381](https://github.com/stepfunc/dnp3/pull/381), [#406](https://github.com/stepfunc/dnp3/pull/406).
* :star: Add `DirectWriteAbsTime` time sync procedure for outstations that don't properly implement IEEE-1815 time sync. See [#403](https://github.com/stepfunc/dnp3/pull/403).
* :star: Add optional support for parsing and transmission of zero-length octet strings via global setting. See [#379](https://github.com/stepfunc/dnp3/pull/379).
* :star: Add optional support for AWS libcrypto (aws-lc-rs) as TLS backend. See [#378](https://github.com/stepfunc/dnp3/pull/378).
* :star: Make OctetString constructor public in Java bindings. See [#402](https://github.com/stepfunc/dnp3/pull/402).
* :star: Make HeaderInfo constructor public for mockability in tests. See [#404](https://github.com/stepfunc/dnp3/pull/404).
* :star: Add `Database::discard_unselected_events()` to drop undelivered events of selected classes on demand. Opt-in and lossy by design. See [#427](https://github.com/stepfunc/dnp3/issues/427).
* :star: Add non-spawning master and outstation task APIs behind the semver-exempt `unstable` feature, for applications that don't want the library calling `tokio::spawn` internally. See [#433](https://github.com/stepfunc/dnp3/pull/433).
* :star: Add `EndpointList::into_connect_handler()` and `into_connect_handler_with_options()` to build a `ClientConnectionHandler` from an endpoint list. See [#433](https://github.com/stepfunc/dnp3/pull/433).
* :shield: Fix inverted `MinTlsVersion` mapping, which affects 1.6.0 through 1.7.0-RC3. See [#437](https://github.com/stepfunc/dnp3/issues/437).
* :bell: **Behavior change for existing TLS configurations.** Users who set `V13` were silently permitting TLS 1.2 and will now reject peers that do not support TLS 1.3. Users on the default `V12` (including all bindings users who never overrode it) will now negotiate TLS 1.3 where the peer supports it, rather than being pinned to TLS 1.2.
* :shield: Update `rustls` to 0.23.45 to resolve [RUSTSEC-2026-0285](https://rustsec.org/advisories/RUSTSEC-2026-0285), in which TLS 1.3 handshake messages were accepted across encryption level boundaries. See [#447](https://github.com/stepfunc/dnp3/pull/447).
* :shield: Update `rustls-webpki` to 0.103.13 to resolve [RUSTSEC-2026-0098](https://rustsec.org/advisories/RUSTSEC-2026-0098) and [RUSTSEC-2026-0099](https://rustsec.org/advisories/RUSTSEC-2026-0099), both concerning incorrect acceptance of X.509 name constraints. Exposure is limited to `CertificateMode::AuthorityBased`.
* :bell: **These two dependency updates only affect the prebuilt binary distributions of the bindings (C/C++, .NET, Java).** Rust consumers of the `dnp3` crate pick up the patched dependencies automatically on rebuild.
* :shield: Add security policy and automated supply chain scanning. See [#394](https://github.com/stepfunc/dnp3/pull/394), [#440](https://github.com/stepfunc/dnp3/pull/440).
* :wrench: Add quality gate to prevent releases when tests fail. See [#393](https://github.com/stepfunc/dnp3/pull/393).
* :wrench: Automate cargo publish to crates.io in CI release process. See [#407](https://github.com/stepfunc/dnp3/pull/407).
* :wrench: Better CLI examples with improved organization and documentation. See [#375](https://github.com/stepfunc/dnp3/pull/375).
* :wrench: Refactor release workflow into separate idempotent jobs for improved reliability.
* :bell: **Rust-only change.** The no-spawn TCP outstation server APIs (`Server::add_outstation_no_spawn` and `Server::bind_no_spawn`) no longer attach a tracing span to the futures they return, so non-spawning callers retain full control over instrumentation. Callers that relied on the automatic `dnp3-outstation-tcp` / `tcp-server` spans should wrap the returned future in their own span. See [#433](https://github.com/stepfunc/dnp3/pull/433).
* :book: Update TLS documentation to clarify empty string behavior for certificate passwords. See [#389](https://github.com/stepfunc/dnp3/pull/389).
* :bug: Reset the outstation event buffer on session teardown. Events left mid-delivery when a link died were previously stranded and never re-reported on reconnect, and stale selected events could leak into a later response. See [#423](https://github.com/stepfunc/dnp3/issues/423).
* :bug: Avoid an index overflow when iterating ranged octet string data whose final index is `u16::MAX`. This only ever tripped the debug-build overflow check after the last item had been yielded; release builds wrapped and produced correct output. See [#422](https://github.com/stepfunc/dnp3/pull/422).
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
