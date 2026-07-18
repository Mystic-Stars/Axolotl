import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

const [command, tag, inputPath] = process.argv.slice(2)
const apiEndpoint = (process.env.CNB_API_ENDPOINT || 'https://api.cnb.cool').replace(/\/$/, '')
const repo = process.env.CNB_REPO_SLUG || 'axlmc/Axolotl'
const token = process.env.CNB_TOKEN
const tokenUser = process.env.CNB_TOKEN_USER_NAME || 'cnb'
const repoUrl = process.env.CNB_REPO_URL_HTTPS || `https://cnb.cool/${repo}.git`

if (!command || !tag || !inputPath || !token) {
	throw new Error(
		'Usage: node cnb-release.mjs <upload|finalize> <tag> <path>; CNB_TOKEN is required',
	)
}

const apiHeaders = {
	Accept: 'application/vnd.cnb.api+json',
	Authorization: `Bearer ${token}`,
}

async function apiRequest(url, options = {}, allowedStatuses = []) {
	const response = await fetch(url, {
		...options,
		headers: {
			...apiHeaders,
			...options.headers,
		},
	})
	if (!response.ok && !allowedStatuses.includes(response.status)) {
		throw new Error(
			`${options.method || 'GET'} ${url} failed (${response.status}): ${await response.text()}`,
		)
	}
	return response
}

async function getRelease() {
	const response = await apiRequest(
		`${apiEndpoint}/${repo}/-/releases/tags/${encodeURIComponent(tag)}`,
		{},
		[404],
	)
	return response.status === 404 ? null : await response.json()
}

async function ensureRelease() {
	const existing = await getRelease()
	if (existing) {
		return existing
	}

	const prerelease = tag.includes('-')
	const response = await apiRequest(
		`${apiEndpoint}/${repo}/-/releases`,
		{
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				tag_name: tag,
				target_commitish: process.env.CNB_COMMIT || tag,
				name: process.env.CNB_TAG_RELEASE_TITLE || `Axolotl Launcher ${tag}`,
				body: process.env.CNB_TAG_RELEASE_DESC || '',
				draft: true,
				prerelease,
				make_latest: 'false',
			}),
		},
		[409],
	)
	if (response.status !== 409) {
		return await response.json()
	}

	for (let attempt = 0; attempt < 10; attempt++) {
		await new Promise((resolve) => setTimeout(resolve, 1000))
		const release = await getRelease()
		if (release) {
			return release
		}
	}
	throw new Error(`Release ${tag} was created concurrently but could not be loaded`)
}

async function uploadAsset(release, filePath) {
	const assetName = path.basename(filePath)
	const size = fs.statSync(filePath).size
	const uploadResponse = await apiRequest(
		`${apiEndpoint}/${repo}/-/releases/${release.id}/asset-upload-url`,
		{
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ asset_name: assetName, size, overwrite: true, ttl: 0 }),
		},
	)
	const upload = await uploadResponse.json()
	const fileResponse = await fetch(upload.upload_url, {
		method: 'PUT',
		body: fs.readFileSync(filePath),
		headers: { 'Content-Type': 'application/octet-stream' },
	})
	if (!fileResponse.ok) {
		throw new Error(
			`Uploading ${assetName} failed (${fileResponse.status}): ${await fileResponse.text()}`,
		)
	}

	const verifyUrl = new URL(upload.verify_url, apiEndpoint).toString()
	await apiRequest(`${verifyUrl}${verifyUrl.includes('?') ? '&' : '?'}ttl=0`, { method: 'POST' })
	console.log(`Uploaded ${assetName}`)
}

async function uploadDirectory(directory) {
	const release = await ensureRelease()
	const files = fs
		.readdirSync(directory, { withFileTypes: true })
		.filter((entry) => entry.isFile())
		.map((entry) => path.join(directory, entry.name))
		.sort((left, right) => {
			const leftIsManifest = path.basename(left).startsWith('latest.')
			const rightIsManifest = path.basename(right).startsWith('latest.')
			return Number(leftIsManifest) - Number(rightIsManifest) || left.localeCompare(right)
		})
	for (const filePath of files) {
		await uploadAsset(release, filePath)
	}
}

async function waitForPlatformManifests() {
	const required = [
		'latest.linux-x64.json',
		'latest.linux-arm64.json',
		'latest.windows-x64.json',
		'latest.macos-universal.json',
	]

	for (let attempt = 0; attempt < 180; attempt++) {
		const release = await getRelease()
		const assets = new Map((release?.assets || []).map((asset) => [asset.name, asset]))
		if (required.every((name) => assets.has(name))) {
			return { release, assets, required }
		}
		console.log(`Waiting for platform manifests (${attempt + 1}/180)`)
		await new Promise((resolve) => setTimeout(resolve, 30_000))
	}
	throw new Error('Timed out waiting for all platform manifests')
}

async function downloadJson(asset) {
	const assetUrl = new URL(asset.url || asset.browser_download_url, apiEndpoint).toString()
	const response = await apiRequest(assetUrl)
	return await response.json()
}

function publishUpdateBranch(manifestPath) {
	if (tag.includes('-')) {
		return
	}

	const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'axolotl-cnb-update-'))
	const auth = Buffer.from(`${tokenUser}:${token}`).toString('base64')
	const git = (...args) => execFileSync('git', args, { cwd: directory, stdio: 'inherit' })
	git('init')
	git('config', 'user.name', 'Axolotl CNB Release')
	git('config', 'user.email', 'build@cnb.cool')
	git('checkout', '--orphan', 'update')
	fs.copyFileSync(manifestPath, path.join(directory, 'latest.json'))
	git('add', 'latest.json')
	git('commit', '-m', `Publish ${tag}`)
	git('remote', 'add', 'origin', repoUrl)
	git(
		'-c',
		`http.extraHeader=Authorization: Basic ${auth}`,
		'push',
		'--force',
		'origin',
		'HEAD:update',
	)
}

async function finalizeRelease(outputDirectory) {
	fs.mkdirSync(outputDirectory, { recursive: true })
	const { release, assets, required } = await waitForPlatformManifests()
	const manifests = await Promise.all(required.map((name) => downloadJson(assets.get(name))))
	const platforms = {}
	for (const manifest of manifests) {
		if (manifest.version !== tag.replace(/^v/, '')) {
			throw new Error(`Platform manifest version ${manifest.version} does not match ${tag}`)
		}
		for (const [platform, update] of Object.entries(manifest.platforms || {})) {
			if (platforms[platform]) {
				throw new Error(`Duplicate platform in manifests: ${platform}`)
			}
			const filename = path.posix.basename(update.url)
			platforms[platform] = {
				...update,
				url: `https://cnb.cool/${repo}/-/releases/download/${encodeURIComponent(tag)}/${encodeURIComponent(filename)}`,
			}
		}
	}

	const requiredPlatforms = [
		'darwin-aarch64',
		'darwin-x86_64',
		'linux-aarch64',
		'linux-x86_64',
		'windows-x86_64',
	]
	for (const platform of requiredPlatforms) {
		const update = platforms[platform]
		if (!update || typeof update.signature !== 'string' || update.signature.trim().length < 32) {
			throw new Error(`Missing signed update for ${platform}`)
		}
	}

	const manifest = {
		version: tag.replace(/^v/, ''),
		notes: process.env.CNB_TAG_RELEASE_DESC || `Axolotl Launcher ${tag}`,
		pub_date: new Date().toISOString(),
		platforms,
	}
	const manifestPath = path.join(outputDirectory, 'latest.json')
	fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`)
	await uploadAsset(release, manifestPath)

	const prerelease = tag.includes('-')
	await apiRequest(`${apiEndpoint}/${repo}/-/releases/${release.id}`, {
		method: 'PATCH',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({
			draft: false,
			prerelease,
			make_latest: prerelease ? 'false' : 'true',
		}),
	})
	publishUpdateBranch(manifestPath)
	console.log(`Published CNB release ${tag}`)
}

if (command === 'upload') {
	await uploadDirectory(inputPath)
} else if (command === 'finalize') {
	await finalizeRelease(inputPath)
} else {
	throw new Error(`Unknown command: ${command}`)
}
