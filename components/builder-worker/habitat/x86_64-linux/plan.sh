# shellcheck disable=SC2034
source "../../../../support/ci/builder-base-plan.sh"
pkg_name=builder-worker
pkg_origin=biome
pkg_maintainer="The Biome Maintainers <humans@biome.sh>"
pkg_license=('Apache-2.0')
pkg_bin_dirs=(bin)
pkg_deps=(core/glibc core/openssl core/gcc-libs core/zeromq
  core/libarchive core/zlib biome/bio biome/bio-studio biome/bio-pkg-export-container
  core/docker core/curl)
pkg_build_deps=(core/make core/cmake core/protobuf-cpp core/protobuf-rust core/coreutils core/cacerts
  core/rust/"$(cat "../../../../rust-toolchain")" core/gcc core/git core/pkg-config)
pkg_binds=(
  [jobsrv]="worker_port worker_heartbeat log_port"
  [depot]="url"
)
pkg_svc_user="root"
pkg_svc_group="root"
bin="bldr-worker"

# Copy hooks/config/default.toml from parent directory so we only maintain
# one copy.
do_begin() {
  mkdir -p hooks
  mkdir -p config
  cp --no-clobber ../_common/run hooks/run
  cp --no-clobber ../_common/config.toml config/config.toml
  cp --no-clobber ../_common/default.toml default.toml
}

do_prepare() {
  do_builder_prepare

  # Used by libssh2-sys
  export DEP_Z_ROOT DEP_Z_INCLUDE
  DEP_Z_ROOT="$(pkg_path_for zlib)"
  DEP_Z_INCLUDE="$(pkg_path_for zlib)/include"

  # Compile the fully-qualified bio cli package identifier into the binary
  PLAN_HAB_PKG_IDENT=$(pkg_path_for bio | sed "s,^$HAB_PKG_PATH/,,")
  export PLAN_HAB_PKG_IDENT
  build_line "Setting PLAN_HAB_PKG_IDENT=$PLAN_HAB_PKG_IDENT"

  # Compile the fully-qualified Studio package identifier into the binary
  PLAN_STUDIO_PKG_IDENT=$(pkg_path_for bio-studio | sed "s,^$HAB_PKG_PATH/,,")
  export PLAN_STUDIO_PKG_IDENT
  build_line "Setting PLAN_STUDIO_PKG_IDENT=$PLAN_STUDIO_PKG_IDENT"

  # Compile the fully-qualified Docker exporter package identifier into the binary
  PLAN_CONTAINER_EXPORTER_PKG_IDENT=$(pkg_path_for bio-pkg-export-container | sed "s,^$HAB_PKG_PATH/,,")
  export PLAN_CONTAINER_EXPORTER_PKG_IDENT
  build_line "Setting PLAN_CONTAINER_EXPORTER_PKG_IDENT=$PLAN_CONTAINER_EXPORTER_PKG_IDENT"

  # Compile the fully-qualified Docker package identifier into the binary
  PLAN_DOCKER_PKG_IDENT=$(pkg_path_for docker | sed "s,^$HAB_PKG_PATH/,,")
  export PLAN_DOCKER_PKG_IDENT
  build_line "Setting PLAN_DOCKER_PKG_IDENT=$PLAN_DOCKER_PKG_IDENT"
}
