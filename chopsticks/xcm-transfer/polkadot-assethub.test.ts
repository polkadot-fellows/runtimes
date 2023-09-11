import { query, tx } from '../../helpers/api'

import { polkadot } from '../../networks/polkadot'
import { statemint } from '../../networks/statemint'

import buildTest from './shared'

const tests = [
  {
    from: 'polkadot',
    to: 'statemint',
    name: 'DOT',
    test: {
      xcmPalletDown: {
        tx: tx.xcmPallet.limitedTeleportAssets(polkadot.dot, 1e12, tx.xcmPallet.parachainV3(0, statemint.paraId)),
        balance: query.balances,
      },
    },
  },
] as const


export type TestType = (typeof tests)[number]

buildTest(tests)
