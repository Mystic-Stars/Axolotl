/**
 * @see https://prettier.io/docs/configuration
 * @type {import("prettier").Config}
 */
module.exports = {
	semi: false,
	singleQuote: true,
	// The repository indents with tabs (.editorconfig + CLAUDE.md); Prettier
	// defaults to spaces, which made `prettier --check` disagree with every
	// tab-indented JSON file.
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
	],
	xmlWhitespaceSensitivity: 'ignore',
}
