// Bring every open PR branch from this fork up to upstream/main with a merge
// (never a rebase), then push. Run after something lands on main so in-flight
// PRs stop failing guardrails against a baseline they have not absorbed yet.
//
//   node scripts/axolotl/sync-open-prs.mjs
//   node scripts/axolotl/sync-open-prs.mjs --dry-run
//   node scripts/axolotl/sync-open-prs.mjs --only 534,546
//
// Requires: git remotes `origin` (fork) and `upstream`, and the `gh` CLI
// authenticated for the fork.

import { spawnSync } from 'node:child_process'

function fail(message) {
	console.error(`error: ${message}`)
	process.exit(1)
}

function git(args, { allowFailure = false } = {}) {
	const result = spawnSync('git', args, { encoding: 'utf8' })
	const output = `${result.stdout ?? ''}${result.stderr ?? ''}`.trim()
	if (result.status !== 0 && !allowFailure) {
		fail(`git ${args.join(' ')} failed:\n${output}`)
	}
	return { status: result.status, output }
}

function gh(args, { allowFailure = false } = {}) {
	const result = spawnSync('gh', args, { encoding: 'utf8' })
	const output = `${result.stdout ?? ''}${result.stderr ?? ''}`.trim()
	if (result.status !== 0 && !allowFailure) {
		fail(`gh ${args.join(' ')} failed:\n${output}`)
	}
	return { status: result.status, output }
}

function parseArgs(argv) {
	const args = { dryRun: false, only: null }
	for (let index = 0; index < argv.length; index += 1) {
		const token = argv[index]
		if (token === '--dry-run') {
			args.dryRun = true
		} else if (token === '--only') {
			const value = argv[index + 1]
			if (!value) fail('--only needs a comma-separated list of PR numbers')
			args.only = new Set(
				value
					.split(',')
					.map((part) => Number(part.trim()))
					.filter((number) => Number.isInteger(number) && number > 0),
			)
			index += 1
		} else if (token === '--help' || token === '-h') {
			console.log(
				[
					'Merge upstream/main into every open PR branch and push.',
					'',
					'  node scripts/axolotl/sync-open-prs.mjs [--dry-run] [--only 534,546]',
				].join('\n'),
			)
			process.exit(0)
		} else {
			fail(`unknown argument: ${token}`)
		}
	}
	return args
}

function currentBranch() {
	return git(['rev-parse', '--abbrev-ref', 'HEAD']).output
}

function originOwner() {
	const url = git(['remote', 'get-url', 'origin']).output
	const match = url.match(/github\.com[/:]([^/]+)\//)
	if (!match) fail(`could not parse an owner from origin remote url: ${url}`)
	return match[1]
}

function openPullRequests() {
	const listed = gh([
		'pr',
		'list',
		'--state',
		'open',
		'--author',
		'@me',
		'--json',
		'number,headRefName,headRepositoryOwner,title',
		'--limit',
		'1000',
	])
	return JSON.parse(listed.output)
}

function behindCount(ref) {
	const counts = git(['rev-list', '--left-right', '--count', `${ref}...upstream/main`], {
		allowFailure: true,
	}).output
	const [ahead, behind] = counts.split(/\s+/).map((part) => Number(part))
	return { ahead: ahead ?? 0, behind: behind ?? 0 }
}

function syncBranch(pr, { dryRun }) {
	const branch = pr.headRefName
	const remoteRef = `origin/${branch}`
	console.log(`\n#${pr.number} ${branch} — ${pr.title}`)

	git(['fetch', 'origin', branch])
	git(['fetch', 'upstream', 'main'])

	if (dryRun) {
		// Never switch or merge in dry-run: only inspect the fetched remote tip.
		const counts = behindCount(remoteRef)
		console.log(`  ahead ${counts.ahead}, behind ${counts.behind} (vs ${remoteRef})`)
		if (counts.behind === 0) {
			console.log('  already up to date')
			return { pr: pr.number, status: 'clean' }
		}
		console.log('  dry-run: would merge upstream/main and push')
		return { pr: pr.number, status: 'would-sync', ...counts }
	}

	git(['switch', branch])
	git(['merge', '--ff-only', remoteRef], { allowFailure: true })

	const counts = behindCount(branch)
	console.log(`  ahead ${counts.ahead}, behind ${counts.behind}`)
	if (counts.behind === 0) {
		console.log('  already up to date')
		return { pr: pr.number, status: 'clean' }
	}

	const merged = git(['merge', 'upstream/main', '--no-edit', '--no-gpg-sign'], {
		allowFailure: true,
	})
	if (merged.status !== 0) {
		console.error(merged.output)
		git(['merge', '--abort'], { allowFailure: true })
		return { pr: pr.number, status: 'conflict', ...counts }
	}

	const pushed = git(['push', 'origin', branch], { allowFailure: true })
	if (pushed.status !== 0) {
		console.error(pushed.output)
		// Leave the local merge in place for inspection, but do not kill the run.
		return { pr: pr.number, status: 'push-failed', ...counts }
	}

	const after = behindCount(branch)
	console.log(`  pushed; ahead ${after.ahead}, behind ${after.behind}`)
	return { pr: pr.number, status: 'synced', ...after }
}

function main() {
	const args = parseArgs(process.argv.slice(2))
	const original = currentBranch()
	const originalStatus = git(['status', '--porcelain']).output
	if (originalStatus !== '') {
		fail('working tree is not clean; commit or stash before syncing PR branches')
	}

	git(['fetch', 'upstream', 'main'])
	const owner = originOwner()
	const prs = openPullRequests().filter((pr) => {
		if (args.only && !args.only.has(pr.number)) return false
		// Only branches that live on this fork's origin remote.
		return pr.headRepositoryOwner?.login === owner
	})

	if (prs.length === 0) {
		console.log('no open pull requests to sync')
		return
	}

	const results = []
	try {
		for (const pr of prs) {
			try {
				results.push(syncBranch(pr, args))
			} catch (error) {
				console.error(`  failed: ${error.message}`)
				results.push({ pr: pr.number, status: 'failed' })
			}
		}
	} finally {
		git(['switch', original], { allowFailure: true })
	}

	console.log('\nsummary')
	for (const result of results) {
		console.log(`  #${result.pr}: ${result.status}`)
	}

	const bad = results.filter((result) =>
		['conflict', 'failed', 'push-failed'].includes(result.status),
	)
	if (bad.length > 0) {
		process.exitCode = 1
	}
}

main()
