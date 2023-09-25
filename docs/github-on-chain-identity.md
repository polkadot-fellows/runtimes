# Adding github account to your on-chain identity

Adding the github account to your identity requires the usage of the `additional` fields in the identity info. This is currently only supported by using the [polkadot-js bare extrinsic interface](https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Frpc.polkadot.io#/extrinsics). See the following image for an example:

![Add github name to the additional fields of the on-chain identity](github-on-chain-identity-process.jpg)

You should also at least add the `display` field. If you want to add `utf8` characters, you need to hex encode them and put the hex string (`0x` prefixed) into the field.
