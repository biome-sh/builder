#!/bin/bash

export HAB_LICENSE="accept-no-persist"

if [ ! -f /bin/bio ]; then
  sudo useradd -r -U hab
  curl https://raw.githubusercontent.com/biome-sh/biome/master/components/bio/install.sh | sudo bash
fi
