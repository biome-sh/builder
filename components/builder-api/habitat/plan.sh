#!/bin/bash
#shellcheck disable=SC2034

source "../../../support/ci/builder-base-plan.sh"

pkg_name=builder-api
pkg_origin=biome
pkg_maintainer="The Biome Maintainers <humans@biome.sh>"
pkg_license=('Apache-2.0')
pkg_bin_dirs=(bin)

pkg_deps=(
  core/coreutils
  core/curl
  core/gcc-base
  core/glibc
  core/libarchive
  core/openssl
  core/postgresql17-client
  core/zeromq
)

pkg_build_deps=(
  core/cacerts
  core/cmake
  core/coreutils
  core/gcc
  core/git
  core/pkg-config
  core/protobuf-cpp
  core/protobuf-rust
  core/rust/"$(tail -n 1 "../../../rust-toolchain" | cut -d'"' -f 2)"
)

pkg_exports=(
  [port]=http.port
)

pkg_exposes=(port)

pkg_binds=(
  [memcached]="port"
)

bin="bldr-api"
