# Unreleased

* Deprecate `error::NoError` in favor of `std::convert::Infallible` ([#688])
* Use `#[non_exhaustive]` for `error::Error`. Note this bumps the minimum supported rust version to 1.40 ([#688]).
* Add the `derive` feature that enables all derive-related smaller features
  (`specs-derive` and `shred-derive` currently). ([#687])

[#687]: https://github.com/amethyst/specs/pull/687
[#688]: https://github.com/amethyst/specs/pull/688

# 0.16.1 (2020-02-18)

* `JoinIter` now implements `Clone` when inner types are `Clone` -- usually for immutable `join()`s. ([#620])
* Bump `hibitset` to `0.6.3`. ([#620])
* `StorageEntry::replace` replaces a component, returning the previous value if any. ([#622])

[#620]: https://github.com/slide-rs/specs/pull/620
[#622]: https://github.com/slide-rs/specs/pull/622

# 0.16.0 (2020-02-13)

* Update `syn`, `quote` and `proc-macro2` to `1.0`. ([#648])
* Implement `ParJoin` for `MaybeJoin` if the inner type is `ParJoin`. ([#655])
* Remove `"nightly"` feature -- improved panic messages are available on stable. ([#671])
* Bump `shred` to `0.10.2`. ([#671], [#674], [#683])
* Components and resources no longer need to be `Send + Sync` if parallel feature is disabled ([#673], [#674])
* Bump `uuid` to `0.8.1`. ([#683])
* Bump `rayon` to `1.3.0`. ([#683])

[#648]: https://github.com/amethyst/specs/pull/648
[#655]: https://github.com/amethyst/specs/pull/655
[#671]: https://github.com/slide-rs/specs/pull/671
[#673]: https://github.com/slide-rs/specs/issues/673
[#674]: https://github.com/slide-rs/specs/pull/674
[#683]: https://github.com/slide-rs/specs/pull/683

# 0.15.1 (2019-09-16)

* Benchmark uses `nalgebra` instead of `cgmath`. ([#619])
* Bumped `shrev` from `1.0` to `1.1`. ([#619]).
* Update hashbrown to 0.6.0, criterion to 0.3 ([#627], [#632])
* Remove `mopa` in favour of `std::any::Any` ([#631])

[#619]: https://github.com/slide-rs/specs/pull/619
[#627]: https://github.com/slide-rs/specs/pull/627
[#631]: https://github.com/slide-rs/specs/pull/631
[#632]: https://github.com/slide-rs/specs/pull/632

# 0.15.0 (2019-06-30)

* Removed `common` and `RudyStorage` ([#542])
* Moved `World` to `shred`, added `WorldExt` trait for Specs functionality ([#550])
* Add `UuidMarker` for UUID <-> `Entity` mappings ([#584])
* Implement `Join` on `BitSetLike` trait object ([#599])
* Expose inner field of `AntiStorage` ([#603])
* Remove `fnv` in favour of `hashbrown` ([#606])
* Reexport `hibitset`, `rayon`, `shred` and `shrev` ([#606])
* Reexport `shred_derive::SystemData` when `shred-derive` feature is enabled ([#606])
* Reexport `specs_derive::{Component, ConvertSaveload}` when `specs-derive` feature is enabled
([#606])

[#542]: https://github.com/slide-rs/specs/pull/542
[#550]: https://github.com/slide-rs/specs/pull/550
[#584]: https://github.com/slide-rs/specs/pull/584
[#599]: https://github.com/slide-rs/specs/pull/599
[#603]: https://github.com/slide-rs/specs/pull/603
[#606]: https://github.com/slide-rs/specs/pull/606

# 0.14.2 (2019-01-04)

* Add `Join`-able entries API to `Storage` ([#518])
* Several docs / meta improvements ([#526], [#527], [#528], [#530], [#531])
* Fix bug when re-killing Entity after atomic killing followed by alloc ([#533])
* Add `Storage::count` and `Storage::is_empty` ([#534])

[#518]: https://github.com/slide-rs/specs/pull/518
[#526]: https://github.com/slide-rs/specs/pull/526
[#527]: https://github.com/slide-rs/specs/pull/527
[#528]: https://github.com/slide-rs/specs/pull/528
[#530]: https://github.com/slide-rs/specs/pull/530
[#531]: https://github.com/slide-rs/specs/pull/531
[#533]: https://github.com/slide-rs/specs/pull/533
[#534]: https://github.com/slide-rs/specs/pull/534

# 0.14.1 (2018-11-23)

* Allow accessing the `EntitiesRes` fetched by a `Storage` ([#515])
* Book / API doc improvements ([#496], [#507], [#511])

[#496]: https://github.com/slide-rs/specs/pull/496
[#507]: https://github.com/slide-rs/specs/pull/507
[#511]: https://github.com/slide-rs/specs/pull/511
[#515]: https://github.com/slide-rs/specs/pull/515

# 0.14.0 (2018-10-28)

* Changed `ConvertSaveload::convert_into` and `ConvertSaveload::convert_from` ([#504])

[#504]: https://github.com/slide-rs/specs/pull/504

# 0.13.0 (2018-10-28)

This release can be skipped; please use `0.14` instead.

* Generation now internally uses the new `NonZeroI32` from `nonzero_signed`, meaning `Option<Entity>` is the same
  size as `Entity`. Note this bumps the minimum supported rust version to 1.28.0 ([#447]).
* Improved `saveload` and added custom derive for components with `Entity` ([#460])
* Removed `world::Bundle` ([#486], [#505])
* Updated Chapter 7: Setup to be more explicit, updated examples to follow that methodology ([#487])
* Added some comments to the `saveload` example ([#492])
* Updated dependency versions ([#494])
* FlaggedStorage rewrite with single event channel instead of multiple for ordering. ([#489])
* Make it possible to run on wasm ([#495])

[#447]: https://github.com/slide-rs/specs/pull/447
[#460]: https://github.com/slide-rs/specs/pull/460
[#486]: https://github.com/slide-rs/specs/pull/486
[#487]: https://github.com/slide-rs/specs/pull/487
[#489]: https://github.com/slide-rs/specs/pull/489
[#492]: https://github.com/slide-rs/specs/pull/492
[#494]: https://github.com/slide-rs/specs/pull/494
[#495]: https://github.com/slide-rs/specs/pull/495
[#505]: https://github.com/slide-rs/specs/pull/505

# 0.12.3 (2018-09-21)

* Add `MaybeJoin` to iterate over components without filtering the joint set ([#455])
* Implement `Join` on `Fetch`/`Read`/`Write`/etc. to eliminate unnecessary dereference ([#472])

[#455]: https://github.com/slide-rs/specs/pull/455
[#472]: https://github.com/slide-rs/specs/pull/472

# 0.12.2 (2018-09-09)

* Fix `Allocator::kill` in the case of atomically created entities. ([#454])

[#454]: https://github.com/slide-rs/specs/pull/454

# 0.12.1 (2018-08-09)

* Add `#[must_use]` to entity builders to avoid people forgetting to call `.build()`. ([#443])

[#443]: https://github.com/slide-rs/specs/pull/443

# 0.12.0 (2018-06-26)

* `Join::open()` and `Storage::unprotected_storage_mut()` have been marked unsafe.
Thanks to [@andrewhickman](https://github.com/andrewhickman) for discovering this
unsoundness!
* Add common `Builder` trait to `EntityBuilder` and `LazyBuilder` ([#426])
* Add common `MarkedBuilder` trait to `EntityBuilder` and `LazyBuilder` ([#426])
* Add `LazyUpdate::exec_mut` which allows adding a resource from a system ([#433])
* Add `build_entity()` to `EntitiesRes` so you can use builder syntax in a system.
* Add `marked()` to LazyBuilder to keep parity with EntityBuilder ([#420])
* Fix `U64MarkerAllocator`'s internal index not being updated on `saveload::DeserializeComponents` ([#420])

[#426]: https://github.com/slide-rs/specs/pull/426
[#420]: https://github.com/slide-rs/specs/pull/420
[#433]: https://github.com/slide-rs/specs/pull/433

# 0.11.2 (2018-05-20)

* Add `unprotected_storage()` and `unprotected_storage_mut()` methods to `Storage` ([#419])

[#419]: https://github.com/slide-rs/specs/pull/419

## 0.11.1 (2018-05-18)

* Add diagrams to book, small code fixes in the book ([#412], [#416], [#417])
* Increase tuple size for `SerializeComponents` / `DeserializeComponents` ([#415])

[#412]: https://github.com/slide-rs/specs/pull/412
[#415]: https://github.com/slide-rs/specs/pull/415
[#416]: https://github.com/slide-rs/specs/pull/416
[#417]: https://github.com/slide-rs/specs/pull/417

## 0.11.0 (2018-05-14)

* Improve docs, book and examples ([#278], [#281], [#283], [#285], [#296], [#313], [#316], [#322], [#350], [#356], [#363])
* Add `StorageEntry` for easier handling of inserting/removing component ([#274])
* Add `EntityBuilder::marked` convenience method ([#287])
* Add `saveload` module for easy entity serialization ([#275], [#337])
* Add `nightly` feature flag for unstable features. ([#290])
* Add `TrackedStorage`, a more ergonomic variant to `FlaggedStorage` ([#291])
* Exclusive/mutable aliasing for getting an `EntityBuilder` to prevent unsafety. ([#294])
* Add `Bundle` for registering multiple resources and components at once. ([#296])
* Add `get()` method to `Join` for retrieving a single entities component in bulk. ([#299])
* Implementations of `Join` for owned `BitSet`s, including `AtomicBitset`. ([#303])
* Remove `FlaggedStorage` (new storage uses the same name) and `TrackedStorage` in favor of the new `Tracked` api. ([#305])
* Add `prelude` module for commonly used structures and traits. ([#305])
* Add `LazyBuilder` for easier entity construction in systems. ([#320])
* Replace `Entry` with `PairedStorage` to prevent runtime checks for `RestrictedStorage`. ([#324])
* Deprecate `check()` which hides a possibly expensive clone. ([#326])
* Add `ChangeSet` for easy application to components. ([#344])
* Use criterion.rs for benchmarks ([#348])
* Update to rayon 1.0 ([#352])
* Add `World::system_data` method ([#369])
* BREAKING: Change the way resources are handled (see below) ([shred#77])
* Export all items currently in prelude in the root of the crate ([#394])
* If an EntityBuilder drops before being built the entity will now be deleted on maintain ([#394])
* Removed some redundancy in documentation ([#394])

There is one bigger breaking change in this release. Almost all`Fetch` / `FetchMut` types need to be replaced
with `Read` / `Write`. Both require the resource to implement `Default`, because now the resources can be
added to the world automatically. If you want to make the resource optional and you don't have a sensible
default, `Option<Read>` / `Option<Write>` can be used. If you absolutely need the resource and it doesn't
work without, use `ReadExpect` which will panic in case the resource does not exist (that's the same
behavior as before).

[#274]: https://github.com/slide-rs/specs/pull/274
[#275]: https://github.com/slide-rs/specs/pull/275
[#278]: https://github.com/slide-rs/specs/pull/278
[#281]: https://github.com/slide-rs/specs/pull/281
[#283]: https://github.com/slide-rs/specs/pull/283
[#285]: https://github.com/slide-rs/specs/pull/285
[#287]: https://github.com/slide-rs/specs/pull/287
[#290]: https://github.com/slide-rs/specs/pull/290
[#291]: https://github.com/slide-rs/specs/pull/291
[#294]: https://github.com/slide-rs/specs/pull/294
[#296]: https://github.com/slide-rs/specs/pull/296
[#297]: https://github.com/slide-rs/specs/pull/297
[#299]: https://github.com/slide-rs/specs/pull/299
[#303]: https://github.com/slide-rs/specs/pull/303
[#305]: https://github.com/slide-rs/specs/pull/305
[#313]: https://github.com/slide-rs/specs/pull/313
[#316]: https://github.com/slide-rs/specs/pull/316
[#320]: https://github.com/slide-rs/specs/pull/320
[#322]: https://github.com/slide-rs/specs/pull/322
[#324]: https://github.com/slide-rs/specs/pull/324
[#326]: https://github.com/slide-rs/specs/pull/326
[#337]: https://github.com/slide-rs/specs/pull/337
[#344]: https://github.com/slide-rs/specs/pull/344
[#348]: https://github.com/slide-rs/specs/pull/348
[#350]: https://github.com/slide-rs/specs/pull/350
[#352]: https://github.com/slide-rs/specs/pull/352
[#356]: https://github.com/slide-rs/specs/pull/356
[#363]: https://github.com/slide-rs/specs/pull/363
[#369]: https://github.com/slide-rs/specs/pull/369
[#394]: https://github.com/slide-rs/specs/pull/394

[shred#77]: https://github.com/slide-rs/shred/pull/77

## 0.10.0 (2017-10-04)

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

## 0.9.3 (2017-07-22)

* Add `specs-derive` crate, custom `#[derive]` for components ([#192])
* Add lazy updates: insert and remove components, execute closures on world ([#214], [#221])

[#192]: https://github.com/slide-rs/specs/pull/192
[#214]: https://github.com/slide-rs/specs/pull/214
[#221]: https://github.com/slide-rs/specs/pull/221

## 0.9.2 (2017-07-02)

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
