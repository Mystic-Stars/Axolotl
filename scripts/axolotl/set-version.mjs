import fs from 'node:fs'

const tag = process.argv[2]
const version = tag?.replace(/^v/, '')

if (!version || !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version)) {
	throw new Error(`Expected a semantic version tag such as v1.2.3, received: ${tag ?? '<none>'}`)
}

const packagePath = 'apps/app-frontend/package.json'
const packageJson = JSON.parse(fs.readFileSync(packagePath, 'utf8'))
packageJson.version = version
fs.writeFileSync(packagePath, `${JSON.stringify(packageJson, null, '\t')}\n`)

for (const cargoPath of ['apps/app/Cargo.toml', 'packages/app-lib/Cargo.toml']) {
	const cargoToml = fs.readFileSync(cargoPath, 'utf8')
	const updated = cargoToml.replace(
		/^(\[package\][\s\S]*?^version\s*=\s*)"[^"]+"/m,
		`$1"${version}"`,
	)

	if (updated === cargoToml) {
		throw new Error(`Could not update package version in ${cargoPath}`)
	}

	fs.writeFileSync(cargoPath, updated)
}

console.log(`Configured Axolotl Launcher ${version}`)
