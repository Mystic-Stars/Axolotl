import fs from 'node:fs'

const [manifestPath, tag] = process.argv.slice(2)
const expectedVersion = tag?.replace(/^v/, '')

if (!manifestPath || !expectedVersion) {
	throw new Error('Usage: node verify-update-manifest.mjs <latest.json> <version-tag>')
}

const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'))

if (manifest.version !== expectedVersion) {
	throw new Error(`Manifest version ${manifest.version} does not match ${expectedVersion}`)
}

const requiredPlatforms = ['darwin-aarch64', 'darwin-x86_64', 'linux-x86_64', 'windows-x86_64']

for (const platform of requiredPlatforms) {
	const update = manifest.platforms?.[platform]

	if (!update || typeof update.signature !== 'string' || update.signature.trim().length < 32) {
		throw new Error(`Missing signed update for ${platform}`)
	}

	const url = new URL(update.url)
	if (
		url.protocol !== 'https:' ||
		url.hostname !== 'github.com' ||
		!url.pathname.toLowerCase().includes('/mystic-stars/axolotl/releases/download/')
	) {
		throw new Error(`Unexpected update URL for ${platform}: ${update.url}`)
	}
}

console.log(`Verified signed updater manifest for ${expectedVersion}`)
