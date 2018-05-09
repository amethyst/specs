# Troubleshooting

## `Tried to fetch a resource, but the resource does not exist.`

This is the most common issue you will face as a new user of Specs. 
This panic will occur whenever a `System` is first dispatched, and one or 
more of the components and/or resources it uses is missing from `World`.

There are a few main reasons for this occurring:

* Forgetting to call `setup` after building a `Dispatcher` or `ParSeq`. Make 
  sure this is always run before the first dispatch.
* Not adding mandatory resources to `World`. You can usually find these by 
  searching for occurrences of `ReadExpect` and `WriteExpect`.
* Manually requesting components/resources from `World` (not inside a `System`),
  where the component/resource is not used by any `System`s, which is most common
  when using the `EntityBuilder`. This is an artifact of how `setup` works, it 
  will only add what is found inside the used `System`s. 
  If you use other components/resources, you need to manually register/add these 
  to `World`.

If all the reasons above have been rejected and the panic still occurs, you 
can find out what exact resource is missing by adding a direct dependency on 
[`shred`][shred], and running the project with the nightly toolchain and enabling
the `nightly` feature on `shred`.

[shred]: https://crates.io/crates/shred
