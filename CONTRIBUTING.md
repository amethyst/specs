# Contributing

First of all, thank you for your interest in contributing.

## Bug reports / feature requests

If you experience any bugs or have feature requests, please [file an issue].

[file an issue]: https://github.com/amethyst/specs/issues/new/choose

## Getting started

If you want to contribute code, please read the following sections.

There are couple of recommended step before you start working on a ticket:

1. If you haven't already, please read [the Specs book](https://specs.amethyst.rs/docs/tutorials/)
2. Please make sure you read our [Code of Conduct](CODE_OF_CONDUCT.md)
3. Refer to the [architecture section](#architecture) below to gain some overview
4. Please continue with the next section (Creating Pull Requests)

## Creating Pull Requests

Once you worked through the [Getting started](#getting-started) section, congrutalations! You can
now start working on [a ticket from the issue tracker][tick]. If there's no ticket yet, please
create one in advance, except your PR provides

* more documentation / minor fixes to existing documentation,
* more tests or
* more benchmarks.

[tick]: https://github.com/amethyst/specs/issues?q=is%3Aissue+is%3Aopen+sort%3Aupdated-desc

Please leave a comment on the respective issue that you're working on it, so we don't end up
with two PRs for the same ticket. While working on a branch, you can refer to the [basic guides]
below in case you are not experienced with Git.

[basic guides]: #git

Once you've made the changes you want to make, or in case you want early feedback / help,
please create a PR. The PR template provides more detail on the last steps required.

## Architecture

Specs exposes an interface for working with the ECS pattern, and it makes heavy use of other
Rust projects to accomplish that.

Specs can be divided into two big parts:

1. execution of code
2. managing data

Number 1 is served by [`shred`](https://github.com/amethyst/shred); it provides the following pieces:

* `System`; this is the central interface for defining logic
* `Dispatcher` and `DispatcherBuilder` - these are responsible for building a plan for how to run systems
  (in parallel & sequentially)

Additionally, `shred` also provides the central piece for number 2:

* `World`; everything that a `System` can access is stored inside.

Specs itself defines component storages (which are also stored inside the `World`).
For those, [`hibitset`](https://github.com/amethyst/hibitset/) is used to:

* store the indices (= entity ids) with an existing component
* allow efficient joining over sparse component storages

More details for the individual components can be found in the respective API documentation.

## Git

This project has some basic guidelines for working with git commits:

* Merge commits are only created by bors; PRs should rebase onto master
  (see the [rebasing section](#dealing-with-upstream-changes) below)

### Cloning the repository

The following sections assume you have cloned the repository as follows:

```sh
git clone https://github.com/amethyst/specs
```

(if you're using SSH, you need to use `git@github.com:amethyst/specs`)

Git by default sets the remote branch you cloned from to `origin`. That's what
is usually used for the fork, so let's change that:

```sh
git remote rename origin upstream
git remote add origin https://github.com/my_user_name/specs
```

(if you're using SSH, you need to use `git@github.com:my_user_name/specs`)

### Starting a new branch

```sh
git fetch upstream && git checkout -b foo upstream/master
```

### Dealing with upstream changes

Please use rebase over merge, since the latter is bad for the commit history.
If you're new to git, here's how to do that:

```sh
git fetch upstream
```

Assuming `upstream` is the upstream repo, this will fetch the latest changes.

Use the following with care if you're new to Git; better make a backup!

```sh
git rebase upstream/master
```

This will try to re-apply your commits on top of the upstream changes. If there
are conflicts, you'll be asked to fix them; once done, add the changes with
`git add -A` and use `git rebase --continue`. Repeat until there are no more
conflicts.

That should be it. Note that you'll have to force-push to your branch in case
you have pushed before.

### Squashing commits

If you created more commits than intended, it can be a good idea to combine some
of your commits. Note that this, again, should be used with care if you don't
know what you're doing; better create a backup before!

```sh
git rebase --interactive HEAD~$num_commits # replace this
```

You just need to replace `num_commits` with the number of commits you want to
edit (use `git log` if unsure).

Now you can simply change some commits to `s` or `f` to merge them into the
above commits. Once done, you'll be asked for the new commit messages.

That should be it. Note that you'll have to force-push to your branch in case
you have pushed before.
