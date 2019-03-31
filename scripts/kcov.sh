#!/bin/sh

docker rm -f mykcov1

# Run image (non-blocking)
docker run -dt --security-opt seccomp=unconfined --name mykcov1 --entrypoint /bin/sh registry.gitlab.com/torkleyy/docker-cargo-kcov || exit 1

#docker cp Cargo.lock mykcov1:/volume
docker cp Cargo.toml mykcov1:/volume
docker cp src/ mykcov1:/volume
docker cp tests/ mykcov1:/volume
docker cp specs-derive/ mykcov1:/volume
docker cp examples/ mykcov1:/volume
docker cp benches/ mykcov1:/volume

docker exec -t mykcov1 /bin/sh -c "cargo check --all" || echo "Failed to generate rustc meta"
docker exec -t mykcov1 /bin/sh -c "cargo kcov --all" || echo "Failed generating report"

rm -R cov
mkdir -p cov
docker cp mykcov1:/volume/target/cov ./

# Force remove image
docker rm -f mykcov1
