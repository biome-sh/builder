pkg_name=testapp3
pkg_origin=neurosis
pkg_version="0.1.0"
pkg_maintainer="The Biome Maintainers <humans@biome.sh>"
pkg_license=("Apache-2.0")
pkg_deps=(core/glibc neurosis/testapp)
pkg_description="This is a dummy app for testing builder APIs"
pkg_upstream_url="https://biome.sh"

do_build() {
    return 0
}

do_install() {
    return 0
}

