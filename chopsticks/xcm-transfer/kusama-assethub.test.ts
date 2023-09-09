import { query, tx } from '../../helpers/api'

import { kusama } from '../../networks/polkadot'
import { statemine } from '../../networks/statemint'

import buildTest from './shared'

const tests = [
  {
    from: 'kusama',
    to: 'statemine',
    name: 'KSM',
    test: {
      xcmPalletDown: {
        tx: tx.xcmPallet.limitedReserveTransferAssetsV3(kusama.ksm, 1e12, tx.xcmPallet.parachainV3(0, statemine.paraId)),
        balance: query.balances,
      },
    },
  },
] as const


export type TestType = (typeof tests)[number]

buildTest(tests)
