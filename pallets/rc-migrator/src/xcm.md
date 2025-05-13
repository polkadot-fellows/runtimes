# XCM configuration changes for AHM

## TODOs:
-[ ] Consider a dedicated migration stage for updating the teleport/reserve location, adjusting total
issuance and checking account balances. This approach prevents XCM teleport locking during the 
entire migration and requires only a two-block lock for the switch.
-[ ] Post migration we need to switch all system chains XCM transport fees beneficiary from
`RelayTreasuryLocation` to the new `AssetHubTreasuryLocation`. Does not necessarily need to be done
_synchronously during AHM_, so let's do it after the migration ends successfully. Besides the
configuration change to use sovereign account of AH Treasury, we will also move the funds to the new
account.

## RC XCM changes

1. DOT/KSM in/out teleports are disabled for RC for the duration of the migration. The reasoning for this
is discussed in detail in [accounts.md](./accounts.md) document.
2. Changed the list of `WaivedLocations` that do not have to pay XCM transport fees:
   - removed `LocalPlurality` from the list,
   - kept System Parachains and local Root (which continue to get free delivery).
3. Did NOT change the destination/beneficiary of XCM delivery/transport fees. They will continue to go
to the local Treasury account, even if the Treasury moves to Asset Hub. See 2nd TODO above for details.

## User Impact (on RC)

Users will not be able to teleport DOT/KSM cross-chain in or out of the Relay chain during the migration.
Note this only affects RC<>SysChains, since with the other parachains we still keep reserve transfers alive.  
Since their accounts are being migrated anyway at a random/non-deterministic point during the migration,
this does not really make much of a difference to UX. Users will likely have to not use their DOT/KSM
accounts/balances during migration.

The rest of the XCM config changes do not affect users.

## AH XCM changes

1. DOT/KSM in/out teleports are disabled for AH for the duration of the migration. The reasoning for this
   is discussed in detail in [accounts.md](./accounts.md) document.
2. Changed the list of locations allowed to execute `UnpaidExecution` XCM instruction:
    - removed `RelayTreasuryLocation` from the list,
    - kept Sibling System Parachains, Relay Chain, Fellowship and Ambassador entities (which continue to
      get free delivery).
3. Changed the list of `WaivedLocations` that do not have to pay XCM transport fees:
    - removed `RelayTreasuryLocation` and Relay chain `Plurality` locations from the list,
    - kept Sibling System Parachains, Relay Chain, Fellowship and Ambassador entities (which continue to
      get free delivery).
4. Changed the destination/beneficiary of XCM delivery/transport fees. Instead of going to RC Treasury
sovereign account, they will go to the local Treasury (new migrated treasury) account.

## User Impact (on AH)

Users will not be able to teleport DOT/KSM cross-chain in or out of Asset Hub during the migration.
Note this only affects AH<>SysChains, since with the other parachains we still keep reserve transfers alive.

The rest of the XCM config changes do not affect users.
