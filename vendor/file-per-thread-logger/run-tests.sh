#!/bin/bash

set -e # Stop on first error.

# Tests should run one after the other because cargo test uses multiple
# threads.

cargo test tests --verbose
cargo test formatted_logs --verbose
cargo test uninitialized_threads_should_panic
cargo test logging_from_uninitialized_threads_allowed
