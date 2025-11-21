#!/usr/bin/env bash

sudo apt update
sudo apt install --assume-yes openssl pkg-config g++ make cmake protobuf-compiler libssl-dev libclang-dev libudev-dev git lz4 yarn

# Free space on the runner
df -h
sudo apt -y autoremove --purge
sudo apt -y autoclean
sudo rm -rf /usr/share/dotnet
sudo rm -rf /opt/ghc
sudo rm -rf "/usr/local/share/boost"
sudo rm -rf "$AGENT_TOOLSDIRECTORY"
df -h

