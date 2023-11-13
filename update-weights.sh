#/bin/bash
cargo build --release -p chain-spec-generator --features runtime-benchmarks
./target/release/chain-spec-generator bridge-hub-polkadot-local --raw >bh-polkadot-local-raw.json
./target/release/chain-spec-generator bridge-hub-kusama-local --raw >bh-kusama-local-raw.json
./target/release/chain-spec-generator asset-hub-polkadot-local --raw >ah-polkadot-local-raw.json
./target/release/chain-spec-generator asset-hub-kusama-local --raw >ah-kusama-local-raw.json

# generic xcm weights
../polkadot-sdk/target/release/polkadot-parachain-benchmarks benchmark pallet \
	--chain bh-polkadot-local-raw.json \
	--pallet pallet-xcm-benchmarks::generic \
	--extrinsic "report_holding,buy_execution,query_response,transact,refund_surplus,set_error_handler,set_appendix,clear_error,descend_origin,clear_origin,report_error,claim_asset,trap,subscribe_version,unsubscribe_version,burn_asset,expect_asset,expect_origin,expect_error,expect_transact_status,query_pallet,report_transact_status,clear_transact_status,set_topic,clear_topic,export_message,set_fees_mode,unpaid_execution,universal_origin" \
	--template=../polkadot-sdk/cumulus/templates/xcm-bench-template.hbs \
	--output=system-parachains/bridge-hubs/bridge-hub-polkadot/src/weights/xcm \
	--no-median-slopes \
	--no-min-squares
../polkadot-sdk/target/release/polkadot-parachain-benchmarks benchmark pallet \
	--chain bh-kusama-local-raw.json \
	--pallet pallet-xcm-benchmarks::generic \
	--extrinsic "report_holding,buy_execution,query_response,transact,refund_surplus,set_error_handler,set_appendix,clear_error,descend_origin,clear_origin,report_error,claim_asset,trap,subscribe_version,unsubscribe_version,burn_asset,expect_asset,expect_origin,expect_error,expect_transact_status,query_pallet,report_transact_status,clear_transact_status,set_topic,clear_topic,export_message,set_fees_mode,unpaid_execution,universal_origin" \
	--template=../polkadot-sdk/cumulus/templates/xcm-bench-template.hbs \
	--output=system-parachains/bridge-hubs/bridge-hub-kusama/src/weights/xcm \
	--no-median-slopes \
	--no-min-squares
../polkadot-sdk/target/release/polkadot-parachain-benchmarks benchmark pallet \
	--chain ah-polkadot-local-raw.json \
	--pallet pallet-xcm-benchmarks::generic \
	--extrinsic "report_holding,buy_execution,query_response,transact,refund_surplus,set_error_handler,set_appendix,clear_error,descend_origin,clear_origin,report_error,claim_asset,trap,subscribe_version,unsubscribe_version,burn_asset,expect_asset,expect_origin,expect_error,expect_transact_status,query_pallet,report_transact_status,clear_transact_status,set_topic,clear_topic,export_message,set_fees_mode,unpaid_execution,universal_origin" \
	--template=../polkadot-sdk/cumulus/templates/xcm-bench-template.hbs \
	--output=system-parachains/asset-hubs/asset-hub-polkadot/src/weights/xcm \
	--no-median-slopes \
	--no-min-squares
../polkadot-sdk/target/release/polkadot-parachain-benchmarks benchmark pallet \
	--chain ah-kusama-local-raw.json \
	--pallet pallet-xcm-benchmarks::generic \
	--extrinsic "report_holding,buy_execution,query_response,transact,refund_surplus,set_error_handler,set_appendix,clear_error,descend_origin,clear_origin,report_error,claim_asset,trap,subscribe_version,unsubscribe_version,burn_asset,expect_asset,expect_origin,expect_error,expect_transact_status,query_pallet,report_transact_status,clear_transact_status,set_topic,clear_topic,export_message,set_fees_mode,unpaid_execution,universal_origin" \
	--template=../polkadot-sdk/cumulus/templates/xcm-bench-template.hbs \
	--output=system-parachains/asset-hubs/asset-hub-kusama/src/weights/xcm \
	--no-median-slopes \
	--no-min-squares
# fungible xcm weights
../polkadot-sdk/target/release/polkadot-parachain-benchmarks benchmark pallet \
	--chain bh-polkadot-local-raw.json \
	--pallet pallet-xcm-benchmarks::fungible \
	--extrinsic "*" \
	--template=../polkadot-sdk/cumulus/templates/xcm-bench-template.hbs \
	--output=system-parachains/bridge-hubs/bridge-hub-polkadot/src/weights/xcm \
	--no-median-slopes \
	--no-min-squares
../polkadot-sdk/target/release/polkadot-parachain-benchmarks benchmark pallet \
	--chain bh-kusama-local-raw.json \
	--pallet pallet-xcm-benchmarks::fungible \
	--extrinsic "*" \
	--template=../polkadot-sdk/cumulus/templates/xcm-bench-template.hbs \
	--output=system-parachains/bridge-hubs/bridge-hub-kusama/src/weights/xcm \
	--no-median-slopes \
	--no-min-squares
../polkadot-sdk/target/release/polkadot-parachain-benchmarks benchmark pallet \
	--chain ah-polkadot-local-raw.json \
	--pallet pallet-xcm-benchmarks::fungible \
	--extrinsic "*" \
	--template=../polkadot-sdk/cumulus/templates/xcm-bench-template.hbs \
	--output=system-parachains/asset-hubs/asset-hub-polkadot/src/weights/xcm \
	--no-median-slopes \
	--no-min-squares
../polkadot-sdk/target/release/polkadot-parachain-benchmarks benchmark pallet \
	--chain ah-kusama-local-raw.json \
	--pallet pallet-xcm-benchmarks::fungible \
	--extrinsic "*" \
	--template=../polkadot-sdk/cumulus/templates/xcm-bench-template.hbs \
	--output=system-parachains/asset-hubs/asset-hub-kusama/src/weights/xcm \
	--no-median-slopes \
	--no-min-squares
