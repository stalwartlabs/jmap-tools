jmap-tools 0.1.9
================================
- Performance improvements.

jmap-tools 0.1.8
================================
- Use `rkyv::primitive::ArchivedU32` and `ArchivedU64` instead of the concrete `rkyv::rend::u32_le` and `rkyv::rend::u64_le`.

jmap-tools 0.1.7
================================
- Fix: Solidus in JSON Pointers break parsing.

jmap-tools 0.1.6
================================
- Parse integers starting with '0' as strings in JSON Pointers.

jmap-tools 0.1.5
================================
- Fixed parsing ids containing digits in JSON Pointers.

jmap-tools 0.1.4
================================
- Added `new` method to `JsonPointer`.

jmap-tools 0.1.3
================================
- Added `as_slice` and `as_mut_slice` methods to `JsonPointer`.

jmap-tools 0.1.2
================================
- Added helper functions.

jmap-tools 0.1.1
================================
- Added helper functions.

jmap-tools 0.1.0
================================
- Initial release.
