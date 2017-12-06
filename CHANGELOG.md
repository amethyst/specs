## [Unreleased]

* Remove `FlaggedStorage` (new storage uses the same name) and `TrackedStorage` in favor of `Tracked` api. ([#305])
* Add `saveload` module for easy entity serialization ([#275])
* Improve docs, book and examples ([#278], [#281], [#283], [#285])
* Add `EntityBuilder::marked` convenience method ([#287])
* Add `StorageEntry` for easier handling of inserting/removing component ([#274])

[#274]: https://github.com/slide-rs/specs/pull/274
[#275]: https://github.com/slide-rs/specs/pull/275
[#278]: https://github.com/slide-rs/specs/pull/278
[#281]: https://github.com/slide-rs/specs/pull/281
[#283]: https://github.com/slide-rs/specs/pull/283
[#285]: https://github.com/slide-rs/specs/pull/285
[#287]: https://github.com/slide-rs/specs/pull/287
[#305]: https://github.com/slide-rs/specs/pull/305

## 0.10.0

* Separate `CheckStorage` into two variants and fix soundness issues ([#203])
* Fix `Merge` system and add test for it ([#243], [#248])
* Add more examples, docs, tests, benchmarks ([#249], [#251], [#254], [#256], [#258])
* Use `Result`s to make Specs more robust ([#260])
* Check code coverage with cargo-travis ([#265])
* Make `common::Errors` atomic and more convenient ([#255], [#262])
* Add `World::delete_all` to clear the world ([#257])
* Fix insertion into occupied `NullStorage` entry ([#269])
* Add `Storage::drain` method ([#273])

[#203]: https://github.com/slide-rs/specs/pull/203
[#243]: https://github.com/slide-rs/specs/pull/243
[#248]: https://github.com/slide-rs/specs/pull/248
[#249]: https://github.com/slide-rs/specs/pull/249
[#251]: https://github.com/slide-rs/specs/pull/251
[#254]: https://github.com/slide-rs/specs/pull/254
[#255]: https://github.com/slide-rs/specs/pull/255
[#256]: https://github.com/slide-rs/specs/pull/256
[#257]: https://github.com/slide-rs/specs/pull/257
[#258]: https://github.com/slide-rs/specs/pull/258
[#260]: https://github.com/slide-rs/specs/pull/260
[#262]: https://github.com/slide-rs/specs/pull/262
[#265]: https://github.com/slide-rs/specs/pull/265
[#273]: https://github.com/slide-rs/specs/pull/273

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
