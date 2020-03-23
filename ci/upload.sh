#!/bin/bash -e

PACKAGES=(
    components/builder-api
    components/builder-api-proxy
    components/builder-datastore
    components/builder-memcached
    components/builder-minio

    components/builder-graph
    components/builder-jobsrv
    components/builder-worker
)

for pkg in "${PACKAGES[@]}"; do
    source "results/$(basename "$pkg").env"

    # shellcheck disable=SC2154
    bio pkg upload results/"$pkg_artifact"
done
