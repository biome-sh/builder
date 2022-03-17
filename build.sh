#!/bin/bash

set -eou pipefail

source ./support/ci/shared.sh

toolchain=$(get_toolchain)

component=${1?component argument required}

# Accept bio license
sudo bio license accept
sudo bio pkg install core/rust/"$toolchain"
sudo bio pkg install core/libarchive
sudo bio pkg install core/openssl
sudo bio pkg install core/zeromq
sudo bio pkg install core/pkg-config
sudo bio pkg install core/protobuf
sudo bio pkg install core/postgresql
sudo bio pkg install core/cmake
# It is important NOT to use a vendored openssl from openssl-sys
# pg-sys does not use openssl-sys. So for components that use
# diesel's postgres feature, you wil end up with 2 versions of openssl
# which can lead to segmentation faults when connecting to postgres
export OPENSSL_NO_VENDOR=1
export LD_RUN_PATH
LD_RUN_PATH="$(bio pkg path core/glibc)/lib:$(bio pkg path core/gcc-libs)/lib:$(bio pkg path core/openssl)/lib:$(bio pkg path core/postgresql)/lib:$(bio pkg path core/zeromq)/lib:$(bio pkg path core/libarchive)/lib"
export PKG_CONFIG_PATH
PKG_CONFIG_PATH="$(bio pkg path core/zeromq)/lib/pkgconfig:$(bio pkg path core/libarchive)/lib/pkgconfig:$(bio pkg path core/postgresql)/lib/pkgconfig:$(bio pkg path core/openssl)/lib/pkgconfig"
eval "$(bio pkg env core/rust/"$toolchain"):$(bio pkg path core/protobuf)/bin:$(bio pkg path core/pkg-config)/bin:$(bio pkg path core/postgresql)/bin:$(bio pkg path core/cmake)/bin:$PATH"

cd "components/$component"
cargo build
