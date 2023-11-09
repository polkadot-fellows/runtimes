#!/usr/bin/env node

import fs from 'fs';
import path from 'path';

const OUTPUT = 'npm_dist/';
const HEADER = `// Copyright (C) 2021-${new Date().getFullYear()} Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
`;

function copyFiles (...files) {
	files.forEach((f) =>
		writeFile(f, fs.readFileSync(f, 'utf-8'))
	);
}

function writeFile (file, contents) {
	fs.writeFileSync(path.join(OUTPUT, file), contents);
}

function writeWithHeader (file, contents) {
	writeFile(file, `${HEADER}\n${contents}`);
}

function mkdirs (...dirs) {
	dirs.forEach((d) => {
		const dir = path.join(OUTPUT, d);

		if (!fs.existsSync(dir)) {
			fs.mkdirSync(dir);
		}
	});
}

function adjustPkg (pkgJson, obj) {
	Object.entries(obj).forEach(([k, v]) => {
		delete pkgJson[k];

		if (v !== undefined) {
			pkgJson[k] = v;
		}
	});
}

function main () {
	const typesD = fs.readFileSync('types.d.ts', 'utf-8');
	const pkgJson = JSON.parse(fs.readFileSync('package.json', 'utf-8'));
	const all = JSON.parse(fs.readFileSync('ss58-registry.json', 'utf-8'));
	const code = JSON.stringify(all.registry, null, '\t');

	adjustPkg(pkgJson, {
		exports: {
			'.': {
				types: './index.d.ts',
				require: './cjs/index.js',
				default: './esm/index.js'
			},
			'./package.json': './package.json',
			'./cjs/package.json': './cjs/package.json',
			'./esm/package.json': './esm/package.json'
		},
		main: './cjs/index.js',
		module: './esm/index.js',
		types: 'index.d.ts',
		type: 'module',
		scripts: undefined,
		devDependencies: undefined
	});

	mkdirs('cjs', 'esm');

	writeWithHeader('cjs/index.js', `module.exports = ${code};\n`);
	writeWithHeader('esm/index.js', `export default ${code};\n`);

	writeFile('cjs/package.json', JSON.stringify({ type: 'commonjs' }, null, '\t'));
	writeFile('esm/package.json', JSON.stringify({ type: 'module' }, null, '\t'));

	writeFile('package.json', JSON.stringify(pkgJson, null, '\t'));
	writeFile('index.d.ts', `${typesD}\ndeclare const _default: Registry;\n\nexport default _default;\n`);

	copyFiles('CHANGELOG.md', 'README.md', 'LICENSE');
}

main();
