#!/bin/bash -e

# We're always trying to update packages if they already installed
bio pkg install -fb ya/bio-sdk
bio pkg install -fb core/git
bio pkg install -fb core/shellcheck
bio pkg install -fb ya/tomlcheck

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

# Package Lint
for pkg in "${PACKAGES[@]}"; do
    bio-plan-tomlcheck "$pkg"
    # bio-plan-shellcheck "$pkg"
    # bio-plan-rendercheck "$pkg"
done

# Building
for pkg in "${PACKAGES[@]}"; do
    bio-plan-build "$pkg"
    # preserve build results for each package
    cp results/last_build.env "results/$(basename "$pkg").env"
done

echo TODO: bio-plan-bats
