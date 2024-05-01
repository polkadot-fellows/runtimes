#!/bin/bash

set -e

trap 'kill -9 -$$ || echo "Environment already teared down"' SIGINT SIGTERM EXIT

test=$1

export LOCAL_BRIDGE_TESTING_PATH=~/local_bridge_testing
export DOWNLOADS_PATH=$LOCAL_BRIDGE_TESTING_PATH/downloads
mkdir -p $DOWNLOADS_PATH

# Download the bridge testing "framework" from the `polkadot-sdk` repo
# to `~/local_bridge_testing/downloads/polkadot-sdk`.
framework_repo_path=$DOWNLOADS_PATH/polkadot-sdk
rm -rf $framework_repo_path
git clone --branch master -n --depth=1 --filter=tree:0 \
  https://github.com/paritytech/polkadot-sdk.git $framework_repo_path
pushd $framework_repo_path
git sparse-checkout set --no-cone bridges/testing/framework
git fetch --tags
git checkout polkadot-v1.11.0
popd
export FRAMEWORK_PATH=$framework_repo_path/bridges/testing/framework
echo

export ZOMBIENET_BINARY=$LOCAL_BRIDGE_TESTING_PATH/bin/zombienet
export POLKADOT_BINARY=/home/serban/workplace/sources/polkadot-sdk/target/release/polkadot
export POLKADOT_PARACHAIN_BINARY=/home/serban/workplace/sources/polkadot-sdk/target/release/polkadot-parachain
export CHAIN_SPEC_GEN_BINARY=$LOCAL_BRIDGE_TESTING_PATH/bin/chain-spec-generator
export SUBSTRATE_RELAY_BINARY=$LOCAL_BRIDGE_TESTING_PATH/bin/substrate-relay

export TEST_DIR=`mktemp -d /tmp/bridges-tests-run-XXXXX`
echo -e "Test folder: $TEST_DIR\n"

${BASH_SOURCE%/*}/tests/$test/run.sh
