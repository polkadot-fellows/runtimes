# Pallet Treasury

The Treasury is migrated along with all existing, active, and inactive proposals and spends. Remote
spends that were previously intended for execution on Asset Hub are now mapped to be executed
locally on Asset Hub. The Treasury on Asset Hub will use Relay Chain blocks as its internal clock,
so all previously established timeouts and periods will remain unchanged.

### Treasury Account

Before migration, the Treasury used two accounts: one (1) on the Relay Chain, derived from the
`PalletId` type and the `py/trsry` byte sequence, and another (2) on Asset Hub, derived from the
Treasury XCM location on the Relay Chain, as seen from Asset Hub (e.g., for Polkadot:
`Location{ parent: 1, X1(PalletInstance(19)) }`). To keep only one Treasury account on Asset Hub,
all assets from account (2) are moved to an account on Asset Hub with the same account id as (1),
and this account will be used for all future spends.
