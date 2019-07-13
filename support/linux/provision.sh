#!/bin/bash
set -eux

curl https://raw.githubusercontent.com/biome-sh/biome/master/components/bio/install.sh | sudo bash
sudo bio install core/busybox-static biome/bio-studio
sudo bio install \
  core/direnv \
  core/wget \
  core/docker \
  core/curl -b
# shellcheck disable=SC2016
echo 'eval "$(direnv hook bash)"' >> ~/.bashrc
