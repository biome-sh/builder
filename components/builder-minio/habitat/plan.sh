#!/bin/bash
# shellcheck disable=SC2034

pkg_name=builder-minio
pkg_origin=biome
pkg_maintainer="The Biome Maintainers <humans@biome.sh>"
pkg_license=('Apache-2.0')
pkg_deps=(core/aws-cli core/bash core/cacerts core/curl core/jq-static core/minio core/openssl)
pkg_build_deps=(core/git)

pkg_exports=(
  [port]=bind_port
  [bucket-name]=bucket_name
  [minio-access-key]=env.MINIO_ACCESS_KEY
  [minio-secret-key]=env.MINIO_SECRET_KEY
)

pkg_version() {
  # TED: After migrating the builder repo we needed to add to
  # the rev-count to keep version sorting working
  echo "$(($(git rev-list HEAD --count) + 5000))"
}

do_before() {
  git config --global --add safe.directory /src
  update_pkg_version
}

do_unpack() {
  return 0
}

do_build() {
  return 0
}

do_install() {
  return 0
}

do_strip() {
  return 0
}
