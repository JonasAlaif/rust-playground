#!/bin/bash

set -euv -o pipefail

tools_to_build="${TOOLS_TO_BUILD-russol}"
perform_push="${PERFORM_PUSH-false}"

repository=shepmaster

crate_api_base=https://crates.io/api/v1/crates

for tool in $tools_to_build; do
    cd "${tool}"

    image_name="${tool}"
    full_name="${repository}/${image_name}"

    docker pull "${full_name}" || true
    docker build -t "${full_name}" \
           .

    docker tag "${full_name}" "${image_name}"

    if [[ "${perform_push}" == 'true' ]]; then
        docker push "${full_name}"
    fi

    cd ..
done
