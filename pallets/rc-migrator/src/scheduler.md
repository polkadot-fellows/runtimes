# Scheduler Pallet

Based on the scheduler pallet's usage in the Polkadot/Kusama runtime, it primarily contains two types of tasks:
1. Tasks from passed referendums
2. Service tasks from the referendum pallet, specifically `nudge_referendum` and `refund_submission_deposit`

We plan to map all calls that are used in the Governance by inspecting the production snapshots.
