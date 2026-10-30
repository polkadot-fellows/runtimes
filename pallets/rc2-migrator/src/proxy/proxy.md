# Proxy migration

Every proxy definition whose permission has a meaning on the Coretime chain is recreated there, under the delegator's address on that chain. The Relay Chain's `Proxies` map is read, not changed.

## What travels

| Case | Rule |
|------|------|
| A definition whose permission the runtime maps (`TryFrom<ProxyType> for PortableProxyType`) | Sent with its delay in Relay Chain blocks. The Coretime chain divides the delay by its block-time ratio, rounding up, releases the `ProxyDeposit` hold the accounts stage placed, and reserves its own deposit for the entry out of that hold, never more. A set that arrives without a hold is recorded with a deposit of 0; the delegator can `poke_deposit` to bring it to that chain's rate. |
| A definition whose permission has no meaning there (staking, governance, …) | Not sent. |
| A pure proxy | Its `Any` definition travels like any other, with no deposit: the creator's was refunded by the accounts stage, so the Coretime entry records 0. The pure's whole balance went to the Coretime chain as free balance, so it can `poke_deposit` through its delegate. `kill_pure` works there with the Relay Chain block and extrinsic index of its creation, since the address derives the same way, unless the spawner is a para's sovereign account, which is named differently there. |
| A para's sovereign account, as delegator or delegate | Named by its sibling address on the wire, like its balance. |
| Announcements | Not migrated. The accounts stage refunded their deposits. |

On Polkadot (October 2026) the map holds 587 entries. 439 are pure proxies, 434 of which already record a deposit of 0 (AHM v1 left their entries in place and moved the creators' deposits to Asset Hub) and 5 record about 20 DOT their creator no longer reserves. The 148 other sets carry a deposit. The common Coretime entry is therefore one recorded with 0.

## What stays

The map is left as it was, deposit fields included, although the accounts stage released every proxy deposit. `Proxy` calls are closed on the Relay Chain from the start of the migration and stay closed after it, so nothing acts on a stale record. Whether to remove the entries or zero the recorded deposits instead is open (`TODO(ahm-v2)` in `migrate_many`).

## On the Coretime chain

`receive_proxies` writes each set into the proxy pallet, merged with the delegator's existing entry and kept in the pallet's order, each set in a transaction of its own. A set that fails is rolled back and parked in `FailedProxies` with its `ProxyDeposit` hold still in place; the rest of the batch continues.

An entry recorded with a deposit of 0 is priced by the proxy pallet on its next change: `add_proxy` and `remove_proxy` reserve the full price of what remains, so a pure without free balance can `remove_proxies` or be killed, but cannot drop one definition at a time until it is funded.
