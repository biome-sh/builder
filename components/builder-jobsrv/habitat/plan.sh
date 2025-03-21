source "../../../support/ci/builder-base-plan.sh"
pkg_name=builder-jobsrv
pkg_origin=biome
pkg_maintainer="The Biome Maintainers <humans@biome.sh>"
pkg_license=('Apache-2.0')
pkg_bin_dirs=(bin)
pkg_deps=(core/glibc core/openssl core/gcc-libs core/zeromq core/libarchive
  core/postgresql)
pkg_build_deps=(core/protobuf-cpp core/protobuf-rust core/coreutils core/cacerts
  core/rust/"$(cat "../../../rust-toolchain")" core/gcc core/git core/pkg-config)
pkg_exports=(
  [worker_port]=net.worker_command_port
  [worker_heartbeat]=net.worker_heartbeat_port
  [log_port]=net.log_ingestion_port
  [rpc_port]=http.port
)
pkg_exposes=(worker_port worker_heartbeat log_port rpc_port)
bin="bldr-jobsrv"
