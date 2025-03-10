## Pallet Proxy

The proxy pallet consists of two storage variables.
## Storage: Proxies

The [Proxies](https://github.com/paritytech/polkadot-sdk/blob/7c5224cb01710d0c14c87bf3463cc79e49b3e7b5/substrate/frame/proxy/src/lib.rs#L564-L579) storage map maps a delegator to its delegates. It can be translated one-to-one by mapping the `ProxyType` and `Delay` fields.
### Proxy Type Translation
The different kinds that are possible for a proxy are a [runtime injected type](https://github.com/paritytech/polkadot-sdk/blob/7c5224cb01710d0c14c87bf3463cc79e49b3e7b5/substrate/frame/proxy/src/lib.rs#L119-L125). Since these are different for each runtime, we need a converter that maps the Relay to AH `ProxyType` as close as possible to keep the original intention. The Relay kind is defined [here](https://github.com/polkadot-fellows/runtimes/blob/dde99603d7dbd6b8bf541d57eb30d9c07a4fce32/relay/polkadot/src/lib.rs#L1000-L1010) and the AH version [here](https://github.com/polkadot-fellows/runtimes/blob/fd8d0c23d83a7b512e721b1fde2ba3737a3478d5/system-parachains/asset-hubs/asset-hub-polkadot/src/lib.rs#L453-L468). This is done by injecting a `RcToProxyType` converter into the Asset Hub migration pallet. This is not bullet proof since it relies on some copy&paste code instead of pulling in the Polkadot runtime into the AH runtime but it is the simplest solution.

Mapping from Relay to AH looks as follows:
- Any: same
- NonTransfer: same
- Governance: newly added
- Staking: newly added
- Variant 4: ignore as it is a historic remnant
- Variant 5: ignore ditto
- CancelProxy: same
- Auction: dropped
- NominationPools: newly added

All variants that serve no purpose anymore on the Relay Chain are deleted from there. For example `Staking`. The ones that are still usable on the relay like `NonTransfer` are **also deleted** since there is no storage deposit taken anymore. (TODO think about what is best here)
### Translation of the Delay

The [delay of a ProxyDefinition](https://github.com/paritytech/polkadot-sdk/blob/7c5224cb01710d0c14c87bf3463cc79e49b3e7b5/substrate/frame/proxy/src/lib.rs#L77) is measured in blocks. These are currently 6 seconds Relay blocks. To translate them to 12s AH blocks, we can divide the number by two.
## Storage: Announcements

The [Announcements](https://github.com/paritytech/polkadot-sdk/blob/7c5224cb01710d0c14c87bf3463cc79e49b3e7b5/substrate/frame/proxy/src/lib.rs#L581-L592) storage maps proxy account IDs to [Accouncement](https://github.com/paritytech/polkadot-sdk/blob/7c5224cb01710d0c14c87bf3463cc79e49b3e7b5/substrate/frame/proxy/src/lib.rs#L80-L89). Since an announcement contains a call hash, we cannot translate them for the same reason as with the Multisigs; the preimage of the hash would be either undecodable, decode to something else (security issue) or accidentally decode to the same thing.  
We therefore do not migrate the announcements.

## Pure Proxy Migration

Pure proxies are [derived](https://github.com/paritytech/polkadot-sdk/blob/4f7a93885e1a35ec60178d3b4f1e59a7df3d85f5/substrate/frame/proxy/src/lib.rs#L790-L806) in the form of `hash(BlockNumber, ParentAcc, ProxyType)`. The important part here is the `ProxyType` enum. The [relay chain](https://github.com/polkadot-fellows/runtimes/blob/main/relay/polkadot/constants/src/lib.rs#L177-L189) and [Asset Hub](https://github.com/polkadot-fellows/runtimes/blob/main/system-parachains/asset-hubs/asset-hub-polkadot/src/lib.rs#L456-L471) have the following variants:

| Index | Relay Chain | Asset Hub | Index Good | Unused on AH | Verdict |
|-------|-------------|-----------|-----------------|-------------|---------|
| 0 | Any | Any | ✅ | ❌ | NoOp |
| 1 | NonTransfer | NonTransfer | ✅ | ✅ | NoOp |
| 2 | Governance | CancelProxy | ❌ | ✅ | Disable on AH |
| 3 | Staking | Assets | ❌ | ✅ | Disable on AH |
| 4 | - | AssetOwner | ✅ | ❌ | NoOp |
| 5 | - | AssetManager | ✅ | ✅ | NoOp |
| 6 | CancelProxy | Collator | ❌ | ✅ | Disable on AH |
| 7 | Auction | TBD | ✅ | ✅ | NoOp |
| 8 | NominationPools | TBD | ✅ | ✅ | NoOp |
| 9 | NominationParaRegistration | TBD | ✅ | ✅ | NoOp |

There is good news and bad news here. The good news is that there is only one account with an `AssetOwner` proxy and that has an index that is unused by the Relay Chain. All other proxies are using `Any`. This means that the case that one proxy imposters as another proxy - by exploiting the colliding enum indices - should not happen. Such an attack could otherwise be devastating, since it could allow one proxy to irrevocably kill another proxy. For example; a `Governance` proxy on the Relay chain could then delete a `CancelProxy` on the asset hub. Deletion (via `Proxy::kill_pure`) is the "only" thing that can be done by this attack.

The bad news is that the proxy pallet does not keep track of pure accounts. There is no storage item for it. I created [a script](https://github.com/ggwpez/substrate-scripts/blob/370b8336f46d6fc5acd2044731874a1e887a2253/proxy-created-events.py) that will print all proxies that are neither `Any` nor `NonTransfer`.  
Polkadot asset hub has a single `AssetOwner` proxy and Kusama Asset Hub none.

We should disable these colliding proxy types on Asset Hub, to avoid future use until the migration finished. But generally it should be possible to migrate the pure proxy accounts as is without any translation of their ID while keeping the ability of them to call `kill_pure`. If the creator account was translated (eg Parachain Sovereign) then they would have to use our account recovery to redeem that account.

We probably also have to add stub variants for the ones that have no equivalent on AH (marked with TBD) to allow users to still redeem these proxies and claim the deposit.

## User Impact
- Announcements need to be re-created
- Proxies of type `Auction` are not migrated and need to be re-created on the Relay
- Existing proxies on Asset Hub will now have more permissions and will be able to access the new pallets as well. For example, the `NonTransfer` proxy will also be able to use nomination pools. This may affect security assumptions of previously created proxies.

## TODO
- What if the owner of a proxy is lost? Then it cannot be re-created by them on the relay.
	- We could do the same as the proxy replication, just in reverse; allowing anyone that can control an account ID on AH to control that same ID on the Relay.
	- Otherwise we have to keep the `NonTransfer` variant alive. But then there is no deposit taken...
