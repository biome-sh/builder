pkg_name=builder-memcached
pkg_origin=biome
pkg_maintainer="The Biome Maintainers <humans@biome.sh>"
pkg_license=('Apache-2.0')
pkg_deps=(core/memcached)
pkg_build_deps=(core/git)
pkg_exports=(
  [port]=port
)
pkg_exposes=(port)
pkg_version() {
  # TED: After migrating the builder repo we needed to add to
  # the rev-count to keep version sorting working
  echo "$(($(git rev-list HEAD --count) + 5000))"
}
do_before() {
  update_pkg_version
}
do_unpack() {
    return 0
}
do_build(){
    return 0
}
do_install() {
    return 0
}
do_strip() {
    return 0
}
