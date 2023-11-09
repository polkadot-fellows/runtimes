#!/usr/bin/env node

import fs from 'fs';
import toml from 'toml';

function main () {
	const pkgJson = JSON.parse(fs.readFileSync('package.json', 'utf-8'));
	const tomlData = toml.parse(fs.readFileSync('Cargo.toml', 'utf-8'));

	pkgJson.version = tomlData.package.version;

	fs.writeFileSync('package.json', JSON.stringify(pkgJson, null, 2));
}

main();
