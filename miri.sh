#!/bin/bash
#
# Convenience script for running Miri, also the same one that the CI runs!

set -e

# use half the available threads since miri can be a bit memory hungry
test_threads=$((($(nproc) - 1) / 2 + 1))
echo using $test_threads threads

# filters out long running tests
filter='not (test(100k) | test(map_test::wrap) | test(map_test::insert_same_key) | test(=mixed_create_merge)| test(=par_join_many_entities_and_systems) | test(=stillborn_entities))'
echo "using filter: \"$filter\""

# Miri currently reports leaks in some tests so we disable that check
# here (might be due to ptr-int-ptr in crossbeam-epoch so might be
# resolved in future versions of that crate).
MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-ignore-leaks" \
    cargo +nightly miri nextest run \
    -E "$filter" \
    --test-threads="$test_threads" \
    # use nocapture or run miri directly to see warnings from miri
    #--nocapture

# Run tests only available when parallel feature is disabled.
MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-ignore-leaks" \
    cargo +nightly miri nextest run \
    --no-default-features \
    -E "binary(no_parallel)" \
    --test-threads="$test_threads"

