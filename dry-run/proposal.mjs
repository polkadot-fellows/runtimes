import { ApiPromise, WsProvider } from '@polkadot/api';
import { blake2AsHex } from '@polkadot/util-crypto';
import { u8aToHex } from '@polkadot/util';

// ---------------------------------------------------------------------------
// CLI parsing
// ---------------------------------------------------------------------------

function usage() {
  console.error(`
Usage: dry-run/proposal.sh [OPTIONS]

Required (one of):
  --preimage-hash <hex>   Preimage hash to look up on the sender chain
  --call-data <hex>       Raw call data hex (0x-prefixed)

Options:
  --sender <ws-url>       Sender chain WS endpoint       [default: ws://localhost:8000]
  --receiver <ws-url>     Receiver chain WS endpoint      (optional — for XCM calls)
  --origin <name>         Scheduler dispatch origin        [default: Root]
                          Examples: Root, WhitelistedCaller, Fellows
  --help                  Show this help
`);
  process.exit(1);
}

function parseArgs() {
  const args = process.argv.slice(2);
  const opts = {
    sender: 'ws://localhost:8000',
    receiver: null,
    origin: 'Root',
    preimageHash: null,
    callData: null,
  };

  for (let i = 0; i < args.length; i++) {
    switch (args[i]) {
      case '--preimage-hash': opts.preimageHash = args[++i]; break;
      case '--call-data':     opts.callData = args[++i]; break;
      case '--sender':        opts.sender = args[++i]; break;
      case '--receiver':      opts.receiver = args[++i]; break;
      case '--origin':        opts.origin = args[++i]; break;
      case '--help':          usage(); break;
      default:
        console.error(`Unknown option: ${args[i]}`);
        usage();
    }
  }

  if (!opts.preimageHash && !opts.callData) {
    console.error('Error: provide either --preimage-hash or --call-data');
    usage();
  }

  return opts;
}

// ---------------------------------------------------------------------------
// Origin mapping — translate a human-readable name to a scheduler origin
// ---------------------------------------------------------------------------

async function schedulerOrigin(api, name) {
  switch (name) {
    case 'Root':
      return { system: 'Root' };
    case 'WhitelistedCaller':
      return { Origins: 'WhitelistedCaller' };
    case 'Fellows':
      // Polkadot Collectives uses FellowshipOrigins, Kusama relay uses Origins
      if (await isParachain(api)) {
        return { FellowshipOrigins: 'Fellows' };
      }
      return { Origins: 'Fellows' };
    default:
      return { Origins: name };
  }
}

// ---------------------------------------------------------------------------
// Detect whether the chain is a parachain (has parachainSystem pallet)
// ---------------------------------------------------------------------------

async function isParachain(api) {
  return !!api.query.parachainSystem;
}

// ---------------------------------------------------------------------------
// Get the correct scheduler block number and clear weight-consuming entries
// ---------------------------------------------------------------------------

async function getSchedulerBlockAndCleanup(api) {
  // incompleteSince is where the scheduler starts scanning. Usually it equals
  // the scheduler's now from on_initialize. On parachains with async backing,
  // two consecutive blocks can share the same relay parent, pushing
  // incompleteSince 1 ahead of lastRelayChainBlockNumber. Clamp to LRCBN so
  // `when <= now` is satisfied on the first produced block.
  const incompleteSince = (await api.query.scheduler.incompleteSince()).unwrap().toNumber();
  let blockNumber = incompleteSince;
  if (await isParachain(api)) {
    const lrcbn = (await api.query.parachainSystem.lastRelayChainBlockNumber()).toNumber();
    blockNumber = Math.min(incompleteSince, lrcbn);
  }
  console.log(`Scheduler block: ${blockNumber} (incompleteSince=${incompleteSince})`);
  // Clear existing entries so periodic tasks don't exhaust block weight.
  const entries = await api.query.scheduler.agenda.entries();
  const clearEntries = [];
  for (const [key] of entries) {
    const blockNum = key.args[0].toNumber();
    if (blockNum !== blockNumber) {
      clearEntries.push([[blockNum], []]);
    }
  }
  if (clearEntries.length > 0) {
    console.log(`Clearing:       ${clearEntries.length} existing agenda entries`);
  }
  return { blockNumber, clearEntries };
}

// ---------------------------------------------------------------------------
// Resolve the call — either from preimage hash or raw call data
// ---------------------------------------------------------------------------

async function resolveCall(api, opts) {
  if (opts.callData) {
    const hex = opts.callData;
    const byteLen = (hex.length - 2) / 2;

    try {
      const call = api.createType('Call', hex);
      console.log(`Call:           ${call.section}.${call.method} (${byteLen} bytes)`);
    } catch {
      console.log(`Call:           unknown (${byteLen} bytes)`);
    }

    // Scheduler's Bounded<Call>::Inline has a max size of 128 bytes.
    // Larger calls must be registered as a preimage and referenced via Lookup.
    if (byteLen <= 128) {
      console.log(`Encoding:       Inline`);
      return { schedulerCall: { Inline: hex }, preimageStorage: null };
    }

    const hash = blake2AsHex(hex, 256);
    console.log(`Encoding:       Lookup (${byteLen} bytes exceeds 128 byte inline limit)`);
    console.log(`Preimage hash:  ${hash}`);

    return {
      schedulerCall: { Lookup: { hash, len: byteLen } },
      preimageStorage: {
        requestStatusFor: [
          [[hash], { Requested: { maybeTicket: null, count: 1, maybeLen: byteLen } }]
        ],
        preimageFor: [
          [[[hash, byteLen]], u8aToHex(api.createType('Bytes', hex).toU8a())]
        ]
      }
    };
  }

  // Look up preimage on chain
  const hash = opts.preimageHash;
  const status = await api.query.preimage.requestStatusFor(hash);
  if (status.isNone) {
    const statusOld = await api.query.preimage.statusFor(hash);
    if (statusOld.isNone) {
      console.error(`Error: preimage ${hash} not found on chain`);
      process.exit(1);
    }
  }

  const statusHuman = (status.isSome ? status : await api.query.preimage.statusFor(hash))
    .unwrap().toHuman();
  const len = parseInt(String(statusHuman?.Unrequested?.len ?? statusHuman?.Requested?.len ?? '0').replace(/,/g, ''));
  if (!len) {
    console.error(`Error: could not determine preimage length from status: ${JSON.stringify(statusHuman)}`);
    process.exit(1);
  }

  // Verify the preimage data exists
  const data = await api.query.preimage.preimageFor([hash, len]);
  if (data.isNone) {
    console.error(`Error: preimage data not found for hash=${hash} len=${len}`);
    process.exit(1);
  }

  try {
    const call = api.createType('Call', data.unwrap().toHex());
    console.log(`Call:           ${call.section}.${call.method} (Lookup, ${len} bytes)`);
  } catch {
    console.log(`Call:           unknown (Lookup, ${len} bytes)`);
  }

  return { schedulerCall: { Lookup: { hash, len } }, preimageStorage: null };
}

// ---------------------------------------------------------------------------
// Print events, highlighting scheduler/XCM/governance events
// ---------------------------------------------------------------------------

const HIGHLIGHT_SECTIONS = new Set([
  'scheduler', 'polkadotXcm', 'xcmpQueue', 'messageQueue',
  'fellowshipCore', 'fellowshipCollective', 'fellowshipReferenda',
  'whitelist', 'system',
]);

function printEvents(events, label) {
  console.log(`\n=== ${label} ===`);
  for (const record of events) {
    const { event, phase } = record;
    const important = HIGHLIGHT_SECTIONS.has(event.section);
    const prefix = important ? '>>>' : '   ';
    console.log(`  ${prefix} [${phase.toString()}] ${event.section}.${event.method}`);
    if (important) {
      console.log(`        ${JSON.stringify(event.data.toHuman())}`);
    }
  }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  const opts = parseArgs();

  console.log('--- Connecting ---');
  const senderApi = await ApiPromise.create({ provider: new WsProvider(opts.sender) });
  const chain = (await senderApi.rpc.system.chain()).toString();
  console.log(`Sender:         ${opts.sender} (${chain})`);

  let receiverApi = null;
  if (opts.receiver) {
    receiverApi = await ApiPromise.create({ provider: new WsProvider(opts.receiver) });
    const rChain = (await receiverApi.rpc.system.chain()).toString();
    console.log(`Receiver:       ${opts.receiver} (${rChain})`);
  }

  console.log(`Origin:         ${opts.origin}`);
  console.log('');

  // Resolve call
  console.log('--- Resolving call ---');
  const { schedulerCall, preimageStorage } = await resolveCall(senderApi, opts);

  // Determine scheduler block number
  console.log('\n--- Preparing scheduler ---');
  const { blockNumber, clearEntries } = await getSchedulerBlockAndCleanup(senderApi);

  const agendaOverride = [
    [
      [blockNumber],
      [{ call: schedulerCall, origin: await schedulerOrigin(senderApi, opts.origin) }]
    ],
    ...clearEntries,
  ];

  const storageOverride = { scheduler: { agenda: agendaOverride, incompleteSince: blockNumber } };
  if (preimageStorage) {
    storageOverride.preimage = preimageStorage;
  }

  await senderApi.rpc('dev_setStorage', storageOverride);
  console.log(`Agenda set at:  block ${blockNumber}`);

  // Produce block on sender
  console.log('\n--- Producing block on sender ---');
  await senderApi.rpc('dev_newBlock');
  const newHeader = await senderApi.rpc.chain.getHeader();
  console.log(`New block:      #${newHeader.number.toNumber()}`);

  const senderEvents = await senderApi.query.system.events();
  printEvents(senderEvents, `${chain} Events`);

  // Produce block on receiver if provided
  if (receiverApi) {
    console.log('\n--- Producing block on receiver ---');
    await receiverApi.rpc('dev_newBlock');
    const rHeader = await receiverApi.rpc.chain.getHeader();
    const rChain = (await receiverApi.rpc.system.chain()).toString();
    console.log(`New block:      #${rHeader.number.toNumber()}`);

    const receiverEvents = await receiverApi.query.system.events();
    printEvents(receiverEvents, `${rChain} Events`);
  }

  // Disconnect
  await senderApi.disconnect();
  if (receiverApi) await receiverApi.disconnect();

  console.log('\n--- Done ---');
  console.log('Chains are still running for inspection.');
}

main().catch((err) => {
  console.error('Error:', err.message || err);
  process.exit(1);
});
