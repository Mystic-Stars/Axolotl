import fs from 'node:fs'

// 从 GitHub release 元数据生成网站下载元数据（tag + asset 文件名列表）。
// 该文件提交到仓库 main 分支后由 CNB 镜像同步，网站从 CNB 拉取，
// 使国内用户无需访问 GitHub API 即可获取下载链接。
const [releasePath, outputPath] = process.argv.slice(2)

if (!releasePath || !outputPath) {
	throw new Error('Usage: node create-website-release-metadata.mjs <release.json> <output.json>')
}

const release = JSON.parse(fs.readFileSync(releasePath, 'utf8'))

if (typeof release.tag_name !== 'string' || !Array.isArray(release.assets)) {
	throw new Error('Release metadata does not contain tag_name and an assets array')
}

const metadata = {
	tag_name: release.tag_name,
	assets: release.assets
		.map((asset) => asset.name)
		.filter((name) => typeof name === 'string' && name.length > 0),
}

fs.writeFileSync(outputPath, `${JSON.stringify(metadata, null, 2)}\n`)
console.log(
	`Wrote ${outputPath} for ${metadata.tag_name} (${metadata.assets.length} assets)`,
)
