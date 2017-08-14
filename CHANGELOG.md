## [Unreleased]

* Make `CheckStorage` wrap a bitset and introduce `RestrictStorage` ([#203])
* Bump rayon (breaking) ([#237])
* Add `SortedStorage`, a storage which keeps a sorted version of components ([#239])

[#203]: https://github.com/slide-rs/specs/pull/203
[#237]: https://github.com/slide-rs/specs/pull/237
[#239]: https://github.com/slide-rs/specs/pull/239

## 0.9.3
* Add `specs-derive` crate, custom `#[derive]` for components ([#192])
* Add lazy updates: insert and remove components, execute closures on world ([#214], [#221])

[#192]: https://github.com/slide-rs/specs/pull/192
[#214]: https://github.com/slide-rs/specs/pull/214
[#221]: https://github.com/slide-rs/specs/pull/221

## 0.9.2
* Fixed grammar in book ([#198])
* Better docs for `World` and better panic message ([#199])
* Add support for Emscripten ([#205])
* Change examples to use `FooStorage<Self>` and destructure system data in method head ([#206])
* `AntiStorage` for `CheckStorage` ([#208])
* Integrate futures by introducing a `common` module ([#209])

[#198]: https://github.com/slide-rs/specs/pull/198
[#199]: https://github.com/slide-rs/specs/pull/199
[#205]: https://github.com/slide-rs/specs/pull/205
[#206]: https://github.com/slide-rs/specs/pull/206
[#208]: https://github.com/slide-rs/specs/pull/208
[#209]: https://github.com/slide-rs/specs/pull/209
[#214]: https://github.com/slide-rs/specs/pull/214

## 0.10.0
* Separate `CheckStorage` into two variants and fix soundness issues ([#203])

[#203]: https://github.com/slide-rs/specs/pull/203
