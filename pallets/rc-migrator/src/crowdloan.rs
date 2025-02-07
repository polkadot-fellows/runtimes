// notes:
// - deposit from the funds map need to be unreserved to the parachain manager https://github.com/paritytech/polkadot-sdk/blob/04847d515ef56da4d0801c9b89a4241dfa827b33/polkadot/runtime/common/src/crowdloan/mod.rs#L416-L417
// - when refunding crowdloan contributions, we need to re-activate the issuance https://github.com/paritytech/polkadot-sdk/blob/04847d515ef56da4d0801c9b89a4241dfa827b33/polkadot/runtime/common/src/crowdloan/mod.rs#L792-L793
// - burn remaining funds in the crowdloan account https://github.com/paritytech/polkadot-sdk/blob/04847d515ef56da4d0801c9b89a4241dfa827b33/polkadot/runtime/common/src/crowdloan/mod.rs#L573-L574

/*

Crowdloan 2008

{
  depositor: 12xWvNmBVt5541brRVANFaFejMrttu8tnBso3NgzsSuZnY7f
  verifier: null
  deposit: 5,000,000,000,000
  raised: 2,220,000,000,000
  end: 21,413,000
  cap: 500,000,000,000,000
  lastContribution: {
    Ending: 21,055,050
  }
  firstPeriod: 17
  lastPeriod: 24
  fundIndex: 91
}

https://polkadot.subscan.io/crowdloan/2008-44

Ended 

*/
