#!/bin/bash

set -eou pipefail

while [[ $# -gt 1 ]]; do
  case $1 in
    -f | --features )       shift
                            features=$1
                            ;;
    -t | --test-options )   shift
                            test_options=$1
                            ;;
    * )                     echo "FAIL SCHOONER"
                            exit 1
  esac
  shift
done

# set the features string if needed
[ -z "${features:-}" ] && features_string="" || features_string="--features ${features}"

component=${1?component argument required}
cargo_test_command="cargo test ${features_string} -- --nocapture ${test_options:-}"

# Accept bio license
sudo bio license accept
sudo bio pkg install core/rust --binlink
sudo bio pkg install core/bzip2
sudo bio pkg install core/libarchive
sudo bio pkg install core/libsodium
sudo bio pkg install core/openssl
sudo bio pkg install core/xz
sudo bio pkg install core/zeromq
sudo bio pkg install core/libpq
sudo bio pkg install core/protobuf --binlink
export SODIUM_STATIC=true # so the libarchive crate links to sodium statically
export LIBARCHIVE_STATIC=true # so the libarchive crate *builds* statically
export OPENSSL_DIR # so the openssl crate knows what to build against
OPENSSL_DIR="$(bio pkg path core/openssl)"
export OPENSSL_STATIC=true # so the openssl crate builds statically
export LIBZMQ_PREFIX
LIBZMQ_PREFIX=$(bio pkg path core/zeromq)
# now include openssl, libpq, and zeromq so they exists in the runtime library path when cargo test is run
export LD_LIBRARY_PATH
LD_LIBRARY_PATH="$(bio pkg path core/libsodium)/lib:$(bio pkg path core/zeromq)/lib:$(bio pkg path core/libpq)/lib"
# include these so that the cargo tests can bind to libarchive (which dynamically binds to xz, bzip, etc), openssl, and sodium at *runtime*
export LIBRARY_PATH
LIBRARY_PATH="$(bio pkg path core/bzip2)/lib:$(bio pkg path core/libsodium)/lib:$(bio pkg path core/openssl)/lib:$(bio pkg path core/libpq)/lib:$(bio pkg path core/xz)/lib"
# setup pkgconfig so the libarchive crate can use pkg-config to fine bzip2 and xz at *build* time
export PKG_CONFIG_PATH
PKG_CONFIG_PATH="$(bio pkg path core/libarchive)/lib/pkgconfig:$(bio pkg path core/libsodium)/lib/pkgconfig:$(bio pkg path core/libpq)/lib/pkgconfig:$(bio pkg path core/openssl)/lib/pkgconfig"

# Set testing filesystem root
export TESTING_FS_ROOT
TESTING_FS_ROOT=$(mktemp -d /tmp/testing-fs-root-XXXXXX)
echo "--- Running cargo test on $component with command: '$cargo_test_command'"
cd "components/$component"
$cargo_test_command
