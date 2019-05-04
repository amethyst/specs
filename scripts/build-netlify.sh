#!/bin/sh

set -e

WORKING_DIR=$(pwd)

# Install everything
mkdir bin
cd bin
install_tar_gz() {
	curl -sL "$1" | tar zxv
}

echo "Installing dependencies..."

install_tar_gz https://github.com/getzola/zola/releases/download/v0.6.0/zola-v0.6.0-x86_64-unknown-linux-gnu.tar.gz
install_tar_gz https://github.com/rust-lang-nursery/mdBook/releases/download/v0.2.1/mdbook-v0.2.1-x86_64-unknown-linux-gnu.tar.gz
BIN_DIR=$(pwd)

# Build website
echo "Building website..."

cd "${WORKING_DIR}"/docs/website
"${BIN_DIR}"/zola build --base-url "${BASE_URL}" -o "${WORKING_DIR}/public"

# Build reference + tutorials
build_book() {
	cd "${WORKING_DIR}"/docs/$1
	"${BIN_DIR}"/mdbook build -d "${WORKING_DIR}/public/docs/$1"
}

echo "Building books..."

build_book reference
build_book tutorials

echo "Done!"
