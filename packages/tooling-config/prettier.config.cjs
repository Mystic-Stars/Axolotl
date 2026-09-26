/**
 * @see https://prettier.io/docs/configuration
 * @type {import("prettier").Config}
 */
module.exports = {
	semi: false,
	singleQuote: true,
	// The repository indents source with tabs (.editorconfig + CLAUDE.md), and
	// the vast majority of tracked JSON is tab-indented too. Prettier defaults
	// to spaces, which made `prettier --check` fail on those files.
	useTabs: true,
	plugins: ['prettier-plugin-toml', 'prettier-plugin-sql-cst', '@prettier/plugin-xml'],
	overrides: [
		{
			files: ['*.jsonc'],
			options: {
				parser: 'jsonc',
				// By spec, JSONC only extends JSON with comment support, not trailing commas as Prettier likes to add
				trailingComma: 'none',
			},
		},
		{
			// TOML is space-indented throughout the repository and is checked by
			// the `tombi` CI job; tabs would fight both.
			files: ['*.toml'],
			options: {
				useTabs: false,
				tabWidth: 2,
			},
		},
	],
	xmlWhitespaceSensitivity: 'ignore',
}
