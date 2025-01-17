#!/bin/bash

set -euv -o pipefail

repository=jonasalaif

for image in rust-stable rust-beta rust-nightly russol rustfmt clippy miri; do
    docker pull "${repository}/${image}"
    # The backend expects images without a repository prefix
    docker tag "${repository}/${image}" "${image}"
done
