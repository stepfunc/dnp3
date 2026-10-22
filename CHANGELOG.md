
### 1.8.0-M2 ###
* :shield: Time out a TLS outstation server's handshake after 5 seconds. The server accepts one connection at a time, so a peer that passed the address filter and then stalled its TLS handshake stopped the server from accepting connections for any of its outstations until the peer disconnected. Sessions already running were unaffected, and plain TCP servers weren't affected. Every release with TLS support is affected. A peer that keeps opening stalled connections can still delay new connections by up to 5 seconds each. See [#532](https://github.com/stepfunc/dnp3/pull/532).
* :shield: Time out a TLS client's handshake after 5 seconds, for masters and outstations that connect with TLS. A server that accepted the connection and then stalled the handshake stopped the client from retrying or moving on to its other endpoints until the server disconnected. See [#533](https://github.com/stepfunc/dnp3/pull/533).
* :shield: Update `rustls-webpki` to 0.103.16 to resolve [RUSTSEC-2026-0382](https://rustsec.org/advisories/RUSTSEC-2026-0382), in which certificate paths through distrusted emSign intermediates could be accepted. dnp3 trusts only the CA file it is configured with, so this requires `CertificateMode::AuthorityBased` with a public emSign root in that file; `CertificateMode::SelfSigned` is not affected. The update only matters for the prebuilt binary distributions of the bindings (C/C++, .NET, Java). See [#505](https://github.com/stepfunc/dnp3/pull/505).
* :shield: File AUTHENTICATE passwords and auth keys are no longer printed by object value decode logging or by the `Debug` output of `FileCredentials` and `AuthKey`. See [#520](https://github.com/stepfunc/dnp3/pull/520).
* :shield: Discard empty UDP datagrams instead of ending the session. A zero-length datagram was treated as the end of the stream, so any host that could reach a bound UDP master or outstation could end its session, and keep it down by repeating it after each reopen. Connected UDP sockets only accept datagrams from their peer. TCP, TLS and serial are not affected. See [#544](https://github.com/stepfunc/dnp3/pull/544).
* :bell: **The minimum supported Rust version is now 1.88.** See [#502](https://github.com/stepfunc/dnp3/pull/502).
* :bell: Deprecate `WriteError::IinError`, `TimeSyncError::IinError`, `TimeSyncError::SystemTimeNotUnix`, `CommandResponseError::Request` and `ObjectParseError::UnsupportedQualifierCode`, which are never returned. They will be removed in 2.0. See [#556](https://github.com/stepfunc/dnp3/pull/556).
* :star: Every public type in the Rust crate implements `Debug`. See [#519](https://github.com/stepfunc/dnp3/pull/519).
* :star: Export `LinkError`, `FrameError` and `LogicError` from `dnp3::link`. `TaskError::Link` contained a `LinkError` that Rust users couldn't name. See [#539](https://github.com/stepfunc/dnp3/pull/539).
* :bug: Fix an outstation resending the first fragment's header and length when a READ is repeated while it waits for confirmation of a later fragment. It now resends the fragment awaiting confirmation. See [#504](https://github.com/stepfunc/dnp3/pull/504).
* :bug: Fix a file reader that aborts in `opened` being told the read failed with `Shutdown`, or with the CLOSE's error, instead of `AbortByUser`. See [#547](https://github.com/stepfunc/dnp3/pull/547).
* :bug: Encode signed integer device attributes of 127 and 32,767 in one and two bytes, the smallest sizes that hold them, instead of the next size up. See [#517](https://github.com/stepfunc/dnp3/pull/517).
* :bug: Report file authentication tasks to the association handler as `FileAuth` instead of `GetFileInfo`. See [#548](https://github.com/stepfunc/dnp3/pull/548).
* :bug: Fix the outstation's event counts when a full event buffer discards an event that was written and awaiting confirmation. The class IIN bits could be wrong until the next confirm. See [#513](https://github.com/stepfunc/dnp3/pull/513).
* :bug: Fix the outstation losing events from an unsolicited response that was never confirmed, because the confirm timed out or DISABLE_UNSOLICITED ended the series. A READ before the next unsolicited retry left them out, and confirming that READ's response deleted them. See [#546](https://github.com/stepfunc/dnp3/pull/546).
* :bug: Fix a connected UDP session ending when its remote port isn't listening yet, as when an outstation starts before its master. On Linux and macOS the ICMP port unreachable reply surfaced as `ConnectionRefused` on the next read or write, which closed the session and dropped traffic until it reopened. See [#524](https://github.com/stepfunc/dnp3/pull/524).
* :bug: Call `connect_failed` on every failed TCP or TLS connection attempt. It was only called when the attempt timed out, not when the connection was refused or the TLS handshake failed. See [#550](https://github.com/stepfunc/dnp3/pull/550).
* :bug: Fix a bound UDP endpoint losing the sender's address for the second and later link frames in a datagram, which ended the session when it replied. See [#510](https://github.com/stepfunc/dnp3/pull/510).
* :bug: Fix the master refreshing the keep-alive of the wrong outstation when several share a channel: a response refreshed the outstation the current task was for, not the one that sent it. See [#553](https://github.com/stepfunc/dnp3/pull/553).
* :bug: Follow the transport reception rules of TB2013-003: a segment that repeats the previous one is discarded without losing the fragment being assembled, and a fragment with no data is no longer passed to the application layer. Affects masters and outstations. See [#508](https://github.com/stepfunc/dnp3/pull/508).
* :bug: Fix the master never timing out a link status request while API messages keep arriving: each message restarted the response timeout. See [#552](https://github.com/stepfunc/dnp3/pull/552).
* :bug: Fix the master reading 1-byte signed integer device attributes as unsigned, for example -1 as 255. See [#500](https://github.com/stepfunc/dnp3/pull/500).
* :bug: Report outstation static points whose flags aren't just ONLINE in a variation with flags, when a variation without flags is requested (counters, frozen counters, analog inputs). See [#484](https://github.com/stepfunc/dnp3/pull/484).
* :bell: Outstations now set IIN2.1 for a read of a point type with no points, and IIN2.2 for a range read that isn't fully installed, consistent with TB2016-001. Masters that explicitly poll types or ranges the outstation doesn't have will now see these errors. See [#487](https://github.com/stepfunc/dnp3/pull/487).
* :bell: Adding an analog input or analog output status with a NaN or negative deadband now fails (`add` returns false) instead of adding a point that never, or always, reports value changes. A deadband written by a master (g34) now accepts positive infinity, which means value changes never produce events. See [#551](https://github.com/stepfunc/dnp3/pull/551).
* :bell: The master now processes a read response that has IIN2.0, IIN2.1 or IIN2.2 set instead of discarding it: the data reaches the `ReadHandler`, events are confirmed, and the rest of a multi-fragment response is read. A read that delivered data succeeds with a warning. One that delivered nothing still fails with `TaskError::RejectedByIin2`, except the startup integrity scan and automatic event scans, which complete so that they don't hold up polling. See [#536](https://github.com/stepfunc/dnp3/pull/536).
* :bell: The master now acts on the IIN1 bits (restart, need time, class events) of a non-read response that it rejects because of IIN2.0, IIN2.1 or IIN2.2, and still fails the task with `TaskError::RejectedByIin2`. See [#554](https://github.com/stepfunc/dnp3/pull/554).
* :bell: Outstations now set IIN2.1 instead of IIN2.0 for an object the function doesn't support, and IIN2.2 for a qualifier the object doesn't support, as TB2016-001 requires. This includes time writes and freezes the application declines with `RequestError::NotSupported`, and analog dead-band writes when they aren't supported. WRITE also accepts 2-byte qualifiers for time and IIN objects. See [#522](https://github.com/stepfunc/dnp3/pull/522).
* :bell: Outstations now set IIN2.1 instead of IIN2.2 for a SELECT, OPERATE or DIRECT_OPERATE that contains an object other than a control, as TB2016-001 requires. See [#541](https://github.com/stepfunc/dnp3/pull/541).
* :bell: Outstations now discard a fragment that isn't a valid request under IEEE 1815-2012 clause 6 (FIR or FIN cleared, UNS set on anything but a confirm, or a response function code) and log a warning, instead of answering with IIN2.0. Unknown function codes still get IIN2.0. See [#525](https://github.com/stepfunc/dnp3/pull/525).
* :bug: Fix a READ that the outstation deferred while waiting for an unsolicited confirm setting IIN2.2 instead of IIN2.1 for an object that can't be read, and silently dropping headers past `max_read_request_headers` instead of setting IIN2.2. The setting's minimum of 64 now applies to deferred READs too. See [#529](https://github.com/stepfunc/dnp3/pull/529).
* :bug: Fix the outstation answering an OPERATE with NO_SELECT when the master confirmed an unsolicited response between the SELECT and the OPERATE ([#501](https://github.com/stepfunc/dnp3/issues/501)). The selection now follows IEEE 1815-2012 Table 4-9: confirms don't end it, a repeated DIRECT_OPERATE can no longer restore it, and a SELECT with the selection's sequence number but different objects is discarded. See [#530](https://github.com/stepfunc/dnp3/pull/530).
* :bug: Fix the outstation dropping the error of a WRITE header when a later header in the same request succeeded. See [#522](https://github.com/stepfunc/dnp3/pull/522).
* :bug: Fix a malformed READ response when a device attribute doesn't fit in the rest of a fragment or can't be encoded. Its object header was left at the end of the fragment; the attribute now moves whole to the next fragment, or is skipped. See [#545](https://github.com/stepfunc/dnp3/pull/545).
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
