<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui/src/composables/i18n.ts'

import AxolotlWordmark from '~/components/brand/AxolotlWordmark.vue'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	home: { id: 'axolotl-site.navigation.home', defaultMessage: 'Axolotl Launcher home' },
	features: { id: 'axolotl-site.navigation.features', defaultMessage: 'Features' },
	faq: { id: 'axolotl-site.navigation.faq', defaultMessage: 'FAQ' },
	downloads: { id: 'axolotl-site.footer.downloads', defaultMessage: 'Downloads' },
	sourceCode: { id: 'axolotl-site.footer.source-code', defaultMessage: 'GitHub' },
	description: {
		id: 'axolotl-site.footer.description',
		defaultMessage: 'A free and open-source Minecraft launcher for Windows, macOS, and Linux.',
	},
	qqGroup: {
		id: 'axolotl-site.footer.qq-group',
		defaultMessage: 'Official QQ group',
	},
	openSourcePrefix: {
		id: 'axolotl-site.footer.open-source-prefix',
		defaultMessage: 'Axolotl is',
	},
	openSourceLink: {
		id: 'axolotl-site.footer.open-source-link',
		defaultMessage: 'open source',
	},
	disclaimer: {
		id: 'axolotl-site.footer.minecraft-disclaimer',
		defaultMessage:
			'NOT AN OFFICIAL MINECRAFT SERVICE. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.',
	},
})
</script>

<template>
	<footer class="site-footer">
		<div class="footer-shell">
			<div class="footer-main">
				<div class="footer-brand">
					<NuxtLink to="/" :aria-label="formatMessage(messages.home)" class="button-animation">
						<AxolotlWordmark />
					</NuxtLink>
					<p>{{ formatMessage(messages.description) }}</p>
					<div class="qq-group">
						<span>{{ formatMessage(messages.qqGroup) }}</span>
						<strong>955605306</strong>
					</div>
				</div>

				<nav class="footer-links" aria-label="Footer">
					<a href="#features">{{ formatMessage(messages.features) }}</a>
					<a href="#faq">{{ formatMessage(messages.faq) }}</a>
					<a href="#download">{{ formatMessage(messages.downloads) }}</a>
					<a href="https://github.com/Mystic-Stars/Axolotl" target="_blank" rel="noopener">
						{{ formatMessage(messages.sourceCode) }}
					</a>
				</nav>
			</div>

			<div class="footer-bottom">
				<p>© {{ new Date().getFullYear() }} Garbage Human Studio</p>
				<p class="open-source">
					{{ formatMessage(messages.openSourcePrefix) }}
					<a href="https://github.com/Mystic-Stars/Axolotl" target="_blank" rel="noopener">
						{{ formatMessage(messages.openSourceLink) }} </a
					>.
				</p>
				<p class="disclaimer">{{ formatMessage(messages.disclaimer) }}</p>
			</div>
		</div>
	</footer>
</template>

<style scoped lang="scss">
.site-footer {
	position: relative;
	border-top: 1px solid var(--color-divider);
	background: color-mix(in srgb, var(--surface-1) 96%, var(--color-brand));
	color: var(--color-base);

	&::before {
		position: absolute;
		top: -1px;
		left: 50%;
		width: min(28rem, 60vw);
		height: 1px;
		transform: translateX(-50%);
		background: linear-gradient(90deg, transparent, var(--color-brand), transparent);
		content: '';
		opacity: 0.6;
	}
}

.footer-shell {
	width: min(80rem, calc(100% - 3rem));
	margin: 0 auto;
	padding: 3.25rem 0 1.5rem;
}

.footer-main {
	display: grid;
	align-items: center;
	grid-template-columns: minmax(0, 1fr) auto;
	gap: 3rem;
}

.footer-brand {
	display: flex;
	align-items: flex-start;
	flex-direction: column;
	gap: 0.7rem;

	p {
		max-width: 30rem;
		margin: 0;
		color: var(--color-secondary);
		font-size: 0.875rem;
		line-height: 1.6;
	}
}

.qq-group {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	padding: 0.45rem 0.7rem;
	border: 1px solid color-mix(in srgb, var(--color-brand) 35%, var(--color-divider));
	border-radius: 0.7rem;
	background: color-mix(in srgb, var(--color-brand) 8%, var(--surface-3));
	font-size: 0.8125rem;

	span {
		color: var(--color-secondary);
	}

	strong {
		color: var(--color-contrast);
		font-variant-numeric: tabular-nums;
		user-select: all;
	}
}

.footer-links {
	display: flex;
	align-items: center;
	flex-wrap: wrap;
	gap: 0.15rem;
	padding: 0.3rem;
	border: 1px solid var(--color-divider);
	border-radius: 0.9rem;
	background: color-mix(in srgb, var(--surface-3) 70%, transparent);

	a {
		color: var(--color-secondary);
		font-size: 0.875rem;
		font-weight: 600;
		text-decoration: none;
		padding: 0.55rem 0.8rem;
		border-radius: 0.65rem;
		transition:
			background 120ms ease,
			color 120ms ease;

		&:hover {
			background: var(--surface-4);
			color: var(--color-contrast);
		}
	}
}

.footer-bottom {
	display: grid;
	align-items: center;
	grid-template-columns: minmax(0, 1fr) auto;
	gap: 0.55rem 2rem;
	margin-top: 2.5rem;
	padding-top: 1.25rem;
	border-top: 1px solid var(--color-divider);
	color: var(--color-secondary);
	font-size: 0.72rem;

	p {
		margin: 0;
	}

	a {
		color: var(--color-base);
		text-underline-offset: 0.15rem;
	}
}

.open-source {
	text-align: right;
}

.disclaimer {
	grid-column: 1 / -1;
	font-size: 0.64rem;
	letter-spacing: 0.015em;
	opacity: 0.45;
}

@media (max-width: 700px) {
	.footer-shell {
		width: calc(100% - 2rem);
		padding-top: 2rem;
	}

	.footer-main {
		align-items: flex-start;
		grid-template-columns: 1fr;
		gap: 2rem;
	}

	.footer-bottom {
		grid-template-columns: 1fr;
		margin-top: 2rem;
		gap: 0.55rem;
	}

	.open-source,
	.disclaimer {
		grid-column: 1;
		text-align: left;
	}
}
</style>
