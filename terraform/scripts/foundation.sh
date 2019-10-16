#!/bin/bash

# Various low-level foundational work needed to set up Biome for our
# Builder environment.

set -eux

sudo mount /dev/xvdf /mnt
echo '/dev/xvdf /mnt     ext4   defaults 0 0' | sudo tee -a /etc/fstab
sudo mkdir -p /mnt/bio
sudo ln -s /mnt/bio /bio

# Add hab user / group
sudo adduser --group hab || echo "Group 'hab' already exists"
sudo useradd -g hab hab || echo "User 'hab' already exists"
