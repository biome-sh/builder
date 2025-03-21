$pkg_name = "builder-worker"
$pkg_origin = "biome"
$pkg_maintainer = "The Biome Maintainers <humans@biome.sh>"
$pkg_license = @("Apache-2.0")
$pkg_deps = @(
    "core/openssl",
    "core/zeromq",
    "core/zlib",
    "core/libarchive",
    "biome/bio",
    "biome/bio-studio",
    "biome/bio-pkg-export-container",
    "core/docker"
)
$pkg_bin_dirs = @("bin")
$pkg_build_deps = @(
    "core/visual-cpp-build-tools-2015",
    "core/protobuf",
    "core/rust/$(Get-Content "../../../../rust-toolchain")",
    "core/cacerts",
    "core/git",
    "core/perl"
)
$pkg_binds = @{
    jobsrv = "worker_port worker_heartbeat log_port"
    depot  = "url"
}
$bin = "bldr-worker"

function pkg_version {
    # TED: After migrating the builder repo we needed to add to
    # the rev-count to keep version sorting working
    5600 + (git rev-list HEAD --count)
}

function Invoke-Before {
    Invoke-DefaultBefore
    Set-PkgVersion
}

function Invoke-Prepare {
    . "$(Get-HabPackagePath visual-cpp-build-tools-2015)\setenv.ps1"
    if ($env:HAB_CARGO_TARGET_DIR) {
        $env:CARGO_TARGET_DIR = "$env:HAB_CARGO_TARGET_DIR"
    }
    else {
        $env:CARGO_TARGET_DIR = "$HAB_CACHE_SRC_PATH\$pkg_dirname"
    }

    $env:SSL_CERT_FILE = "$(Get-HabPackagePath "cacerts")/ssl/certs/cacert.pem"
    $env:LIB += ";$HAB_CACHE_SRC_PATH/$pkg_dirname/lib"
    $env:INCLUDE += ";$HAB_CACHE_SRC_PATH/$pkg_dirname/include"
    $env:LIBARCHIVE_INCLUDE_DIR = "$(Get-HabPackagePath "libarchive")/include"
    $env:LIBARCHIVE_LIB_DIR = "$(Get-HabPackagePath "libarchive")/lib"
    $env:OPENSSL_NO_VENDOR = 1
    $env:OPENSSL_LIB_DIR = "$(Get-HabPackagePath "openssl")/lib"
    $env:OPENSSL_INCLUDE_DIR = "$(Get-HabPackagePath "openssl")/include"
    $env:LIBZMQ_PREFIX = "$(Get-HabPackagePath "zeromq")"

    # Used by the `build.rs` program to set the version of the binaries
    $env:PLAN_VERSION = "$pkg_version/$pkg_release"
    Write-BuildLine "Setting env:PLAN_VERSION=$env:PLAN_VERSION"

    # Used to set the active package target for the binaries at build time
    $env:PLAN_PACKAGE_TARGET = "$pkg_target"
    Write-BuildLine "Setting env:PLAN_PACKAGE_TARGET=$env:PLAN_PACKAGE_TARGET"

    # Compile the fully-qualified bio package identifier into the binary
    $env:PLAN_HAB_PKG_IDENT = $(Get-HabPackagePath "bio").replace("$HAB_PKG_PATH\","").replace("\", "/")
    Write-BuildLine "Setting env:PLAN_HAB_PKG_IDENT=$env:PLAN_HAB_PKG_IDENT"

    # Compile the fully-qualified Studio package identifier into the binary
    $env:PLAN_STUDIO_PKG_IDENT = $(Get-HabPackagePath "bio-studio").replace("$HAB_PKG_PATH\","").replace("\", "/")
    Write-BuildLine "Setting env:PLAN_STUDIO_PKG_IDENT=$env:PLAN_STUDIO_PKG_IDENT"

    # Compile the fully-qualified Docker exporter package identifier into the binary
    $env:PLAN_CONTAINER_EXPORTER_PKG_IDENT = $(Get-HabPackagePath "bio-pkg-export-container").replace("$HAB_PKG_PATH\","").replace("\", "/")
    Write-BuildLine "Setting env:PLAN_CONTAINER_EXPORTER_PKG_IDENT=$env:PLAN_CONTAINER_EXPORTER_PKG_IDENT"
}

function Invoke-BuildConfig {
    Invoke-DefaultBuildConfig
    Write-BuildLine "Creating config and hooks directories"
    New-Item -ItemType Directory -Force -Path "$pkg_prefix/hooks" | Out-Null
    New-Item -ItemType Directory -Force -Path "$pkg_prefix/config" | Out-Null
    Write-BuildLine "Copying run.ps1 to run"
    Copy-Item "$PLAN_CONTEXT/hooks/run.ps1" "$pkg_prefix/hooks/run"
    Write-BuildLine "Copying default.toml into $pkg_prefix"
    Copy-Item "$PLAN_CONTEXT/../_common/default.toml" "$pkg_prefix/default.toml"
    Write-BuildLine "Copying config.toml into $pkg_prefix/config"
    Copy-Item "$PLAN_CONTEXT/../_common/config.toml" "$pkg_prefix/config/config.toml"
}

function Invoke-Build {
    Push-Location "$PLAN_CONTEXT"
    try {
        cargo build --release --verbose
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Cargo build failed!"
        }
    }
    finally { Pop-Location }
}

function Invoke-Install {
    Write-BuildLine "$HAB_CACHE_SRC_PATH/$pkg_dirname"
    Copy-Item "$env:CARGO_TARGET_DIR/release/bldr-worker.exe" "$pkg_prefix/bin/bldr-worker.exe"
    Copy-Item "$(Get-HabPackagePath "openssl")/bin/*.dll" "$pkg_prefix/bin"
    Copy-Item "$(Get-HabPackagePath "zlib")/bin/*.dll" "$pkg_prefix/bin"
    Copy-Item "$(Get-HabPackagePath "libarchive")/bin/*.dll" "$pkg_prefix/bin"
    Copy-Item "$(Get-HabPackagePath "zeromq")/bin/*.dll" "$pkg_prefix/bin"
    Copy-Item "$(Get-HabPackagePath "visual-cpp-build-tools-2015")/Program Files/Microsoft Visual Studio 14.0/VC/redist/x64/Microsoft.VC140.CRT/*.dll" "$pkg_prefix/bin"
}
