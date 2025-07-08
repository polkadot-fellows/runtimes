# Pallet Nomination Pools

The nomination pools pallet has 15 storage items of which 14 can be migrated without any
translation.

# Storage Values

All nine storage values are migrated as is in a single message.

# Storage Maps

The storage maps are migrated as it. On the receiving side the block number provider has to be set
to Relay Chain Block number provider.

## User Impact

Impact here is negligible and only for pool operators - not members:
- Pool commission change rate (measured in blocks) could be decreased by one block.
- Pool operators may be able to change the commission rate one block later than anticipated. This is
  due to the nature or translating blocks of two different blockchains which does not yield
  unambiguous results.
