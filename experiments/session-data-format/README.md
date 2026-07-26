# `session-data-format`

A host-runnable copy of `src/secret/session_data.rs`, the platform-independent
serialization of the sessions used by the Android secret backend.

That module is `#[cfg(any(target_os = "android", test))]` and lives inside a
crate that needs the full GTK stack to build, which is not always available on a
development host. This experiment mirrors the fields of `StoredSession` and
includes the module verbatim (through `#[path]`), so the format and its unit
tests can be exercised directly:

```sh
cargo test
```

`src/session_data.rs` is generated from the real module with only the module
path and visibility adapted:

```sh
sed -e 's/^use super::StoredSession;/use crate::StoredSession;/' \
    -e 's/pub(super)/pub(crate)/g' \
    ../../src/secret/session_data.rs > src/session_data.rs
```
