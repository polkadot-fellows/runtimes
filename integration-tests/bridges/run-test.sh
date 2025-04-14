#!/bin/bash

set -e

trap 'kill -9 -$$ || echo "Environment already teared down"' SIGINT SIGTERM EXIT

test=$1

export LOCAL_BRIDGE_TESTING_PATH=~/local_bridge_testing

if [ -z "$FRAMEWORK_REPO_PATH" ]; then
  # Download the bridge testing "framework" from the `polkadot-sdk` repo
  # to `~/local_bridge_testing/downloads/polkadot-sdk`.
  export DOWNLOADS_PATH=$LOCAL_BRIDGE_TESTING_PATH/downloads
  echo "FRAMEWORK_REPO_PATH is NOT set, so downloading 'polkadot-sdk' repo to the: $DOWNLOADS_PATH"
  mkdir -p $DOWNLOADS_PATH
  framework_repo_path=$DOWNLOADS_PATH/polkadot-sdk
  rm -rf $framework_repo_path
  git clone --branch master -n --depth=1 --filter=tree:0 \
    https://github.com/paritytech/polkadot-sdk.git $framework_repo_path
  pushd $framework_repo_path
  git sparse-checkout set --no-cone bridges/testing/framework
  git fetch --tags
  git checkout polkadot-stable2409
  popd
else
    framework_repo_path=$FRAMEWORK_REPO_PATH
fi

export FRAMEWORK_PATH=$framework_repo_path/bridges/testing/framework
echo "Using bridges testing framework from path: $FRAMEWORK_PATH"
echo

export ZOMBIENET_BINARY=$LOCAL_BRIDGE_TESTING_PATH/bin/zombienet
export POLKADOT_BINARY=$LOCAL_BRIDGE_TESTING_PATH/bin/polkadot
export POLKADOT_PARACHAIN_BINARY=$LOCAL_BRIDGE_TESTING_PATH/bin/polkadot-parachain
export CHAIN_SPEC_GEN_BINARY_FOR_KUSAMA=$LOCAL_BRIDGE_TESTING_PATH/bin/chain-spec-generator-kusama
export CHAIN_SPEC_GEN_BINARY_FOR_POLKADOT=$LOCAL_BRIDGE_TESTING_PATH/bin/chain-spec-generator-polkadot
export SUBSTRATE_RELAY_BINARY=$LOCAL_BRIDGE_TESTING_PATH/bin/substrate-relay

export TEST_DIR=`mktemp -d /tmp/bridges-tests-run-XXXXX`
echo -e "Test folder: $TEST_DIR\n"

${BASH_SOURCE%/*}/tests/$test/run.sh
