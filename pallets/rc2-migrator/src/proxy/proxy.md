# Proxy migration

Every proxy definition whose permission has a meaning on the Coretime chain is recreated there, under the delegator's address on that chain. The Relay Chain's `Proxies` map is read, not changed.

## What travels

| Case | Rule |
|------|------|
| A definition whose permission the runtime maps (`TryFrom<ProxyType> for PortableProxyType`) | Sent with its delay in Relay Chain blocks. The Coretime chain converts the delay to its own blocks and takes its own deposit out of the `ProxyDeposit` hold the accounts stage placed. |
| A definition whose permission has no meaning there (staking, governance, …) | Not sent. |
| A pure proxy | Its `Any` definition travels like any other. The pure account can only act through it, which is why the accounts stage routed its whole balance to the Coretime chain. |
| A para's sovereign account, as delegator or delegate | Named by its sibling address on the wire, like its balance. |
| Announcements | Not migrated. The accounts stage refunded their deposits. |

## What stays

The map is left as it was, deposit fields included, although the accounts stage released every proxy deposit. Nothing can act on a stale record: `Proxy` calls are closed on the Relay Chain from the start of the migration and stay closed after it.
