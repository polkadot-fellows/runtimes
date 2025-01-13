## Pallet Proxy

The proxy pallet consists of two storage variables.
## Storage: Proxies

The [Proxies](https://github.com/paritytech/polkadot-sdk/blob/7c5224cb01710d0c14c87bf3463cc79e49b3e7b5/substrate/frame/proxy/src/lib.rs#L564-L579) storage map maps a delegator to its delegates. It can be translated one-to-one by mapping the `ProxyType` an `Delay` fields.
### Proxy Type Translation
The different kinds that are possible for a proxy are a [runtime injected type](https://github.com/paritytech/polkadot-sdk/blob/7c5224cb01710d0c14c87bf3463cc79e49b3e7b5/substrate/frame/proxy/src/lib.rs#L119-L125). Since these are different for each runtime, we need a converter that maps the Relay to AH `ProxyType` as close as possible to keep the original intention. The Relay kind is defined [here](https://github.com/polkadot-fellows/runtimes/blob/dde99603d7dbd6b8bf541d57eb30d9c07a4fce32/relay/polkadot/src/lib.rs#L1000-L1010) and the AH version [here](https://github.com/polkadot-fellows/runtimes/blob/fd8d0c23d83a7b512e721b1fde2ba3737a3478d5/system-parachains/asset-hubs/asset-hub-polkadot/src/lib.rs#L453-L468).

Mapping from Relay to AH looks as follows:
- Any: same
- NonTransfer: same
- Governance: needs to be added
- Staking: needs to be added
- Variant 4: ignore as it is a historic remnant
- Variant 5: ignore ditto
- CancelProxy: same
- Auction: ?
- NominationPools: needs to be added

All variants that serve no purpose anymore on the Relay Chain are deleted from there. For example `Stakin`. The ones that are still usable on the relay like `NonTransfer` are **also deleted** since there is no storage deposit taken anymore. (TODO think about what is best here)
### Delay Translation

The [delay of a ProxyDefinition](https://github.com/paritytech/polkadot-sdk/blob/7c5224cb01710d0c14c87bf3463cc79e49b3e7b5/substrate/frame/proxy/src/lib.rs#L77) is measured in blocks. These are currently 6 seconds Relay blocks. To translate them to 12s AH blocks, we can divide the number by two.
## Storage: Announcements

The [Announcements](https://github.com/paritytech/polkadot-sdk/blob/7c5224cb01710d0c14c87bf3463cc79e49b3e7b5/substrate/frame/proxy/src/lib.rs#L581-L592) storage maps proxy account IDs to [Accouncement](https://github.com/paritytech/polkadot-sdk/blob/7c5224cb01710d0c14c87bf3463cc79e49b3e7b5/substrate/frame/proxy/src/lib.rs#L80-L89). Since an announcement contains a call hash, we cannot translate them for the same reason as with the Multisigs; the preimage of the hash would be either undecodable, decode to something else (security issue) or accidentally decode to the same thing.  

We therefore do not migrate the announcements.
## User Impact
- Announcements need to be re-created
- Proxies of type `Auction` are not migrated and need to be re-created on the Relay
