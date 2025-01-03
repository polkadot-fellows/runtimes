#!/bin/bash

# Test that checks if asset transfer works on P<>K bridge.
# This test is intentionally not added to the CI. It is meant to be ran manually.

set -e

source "$FRAMEWORK_PATH/utils/common.sh"
source "$FRAMEWORK_PATH/utils/zombienet.sh"

export ENV_PATH=`realpath ${BASH_SOURCE%/*}/../../environments/polkadot-kusama`

$ENV_PATH/spawn.sh --init --start-relayer &
env_pid=$!

ensure_process_file $env_pid $TEST_DIR/polkadot.env 600
polkadot_dir=`cat $TEST_DIR/polkadot.env`
echo

ensure_process_file $env_pid $TEST_DIR/kusama.env 300
kusama_dir=`cat $TEST_DIR/kusama.env`
echo

run_zndsl ${BASH_SOURCE%/*}/dot-reaches-kusama.zndsl $kusama_dir
run_zndsl ${BASH_SOURCE%/*}/ksm-reaches-polkadot.zndsl $polkadot_dir

run_zndsl ${BASH_SOURCE%/*}/wdot-reaches-polkadot.zndsl $polkadot_dir
run_zndsl ${BASH_SOURCE%/*}/wksm-reaches-kusama.zndsl $kusama_dir
