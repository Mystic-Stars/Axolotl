import fs from 'node:fs'
import path from 'node:path'

const [tag, buildName, platformList, bundleDirectory, outputDirectory] = process.argv.slice(2)

if (!tag || !buildName || !platformList || !bundleDirectory || !outputDirectory) {
	throw new Error(
		'Usage: node prepare-cnb-release.mjs <tag> <build-name> <platforms> <bundle-dir> <output-dir>',
	)
}

const version = tag.replace(/^v/, '')
const platforms = platformList.split(',').filter(Boolean)

function listFiles(directory) {
	return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
		const entryPath = path.join(directory, entry.name)
		return entry.isDirectory() ? listFiles(entryPath) : [entryPath]
	})
}

function isUpdaterArtifact(filePath) {
	if (platforms.every((platform) => platform.startsWith('darwin-'))) {
		return filePath.endsWith('.app.tar.gz')
	}
	if (platforms.every((platform) => platform.startsWith('windows-'))) {
		return filePath.endsWith('.nsis.zip')
	}
	if (platforms.every((platform) => platform.startsWith('linux-'))) {
		return filePath.endsWith('.AppImage.tar.gz')
	}
	return false
}

const releaseAssetSuffixes = [
	'.AppImage',
	'.AppImage.tar.gz',
	'.deb',
	'.dmg',
	'.exe',
	'.msi',
	'.nsis.zip',
	'.rpm',
	'.sig',
	'.app.tar.gz',
]
const files = listFiles(bundleDirectory).filter((filePath) =>
	releaseAssetSuffixes.some((suffix) => filePath.endsWith(suffix)),
)
const updaterArtifacts = files.filter(isUpdaterArtifact)
if (updaterArtifacts.length !== 1) {
	throw new Error(
		`Expected one updater artifact for ${platformList}, found ${updaterArtifacts.length}: ${updaterArtifacts.join(', ')}`,
	)
}

const updaterArtifact = updaterArtifacts[0]
const signaturePath = `${updaterArtifact}.sig`
if (!fs.existsSync(signaturePath)) {
	throw new Error(`Missing updater signature: ${signaturePath}`)
}

fs.mkdirSync(outputDirectory, { recursive: true })
const copiedNames = new Set()
for (const filePath of files) {
	const filename = path.basename(filePath)
	if (copiedNames.has(filename)) {
		throw new Error(`Duplicate release filename: ${filename}`)
	}
	copiedNames.add(filename)
	fs.copyFileSync(filePath, path.join(outputDirectory, filename))
}

const updaterFilename = path.basename(updaterArtifact)
const signature = fs.readFileSync(signaturePath, 'utf8').trim()
const manifest = {
	version,
	notes: process.env.CNB_TAG_RELEASE_DESC || `Axolotl Launcher ${tag}`,
	pub_date: new Date().toISOString(),
	platforms: Object.fromEntries(
		platforms.map((platform) => [
			platform,
			{
				signature,
				url: updaterFilename,
			},
		]),
	),
}

fs.writeFileSync(
	path.join(outputDirectory, `latest.${buildName}.json`),
	`${JSON.stringify(manifest, null, 2)}\n`,
)
