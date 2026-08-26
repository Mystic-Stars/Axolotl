<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui/src/composables/i18n.ts'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	features: { id: 'axolotl-site.navigation.features', defaultMessage: 'Features' },
	faq: { id: 'axolotl-site.navigation.faq', defaultMessage: 'FAQ' },
	downloads: { id: 'axolotl-site.footer.downloads', defaultMessage: 'Downloads' },
	changelog: { id: 'axolotl-site.footer.release-history', defaultMessage: 'Changelog' },
	terms: { id: 'axolotl-site.navigation.terms', defaultMessage: 'Terms of Service' },
	privacy: { id: 'axolotl-site.navigation.privacy', defaultMessage: 'Privacy Policy' },
	developer: { id: 'axolotl-site.footer.developer', defaultMessage: 'Axolotl Launcher Team' },
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
	<footer class="site-footer relative border-t border-divider bg-surface-1 text-[var(--color-base)]">
		<div class="footer-inner">
			<div class="footer-legal flex flex-wrap gap-x-[1.4rem] gap-y-[0.4rem] text-[var(--color-secondary)] text-[0.78rem]">
				<p>© {{ new Date().getFullYear() }} {{ formatMessage(messages.developer) }}</p>
				<p class="open-source">
					{{ formatMessage(messages.openSourcePrefix) }}
					<a href="https://github.com/Mystic-Stars/Axolotl" target="_blank" rel="noopener">
						{{ formatMessage(messages.openSourceLink) }} </a
					>.
				</p>
			</div>

			<nav class="footer-links flex flex-wrap justify-end gap-x-[1.3rem] gap-y-[0.35rem]" aria-label="Footer">
				<NuxtLink to="/#features">{{ formatMessage(messages.features) }}</NuxtLink>
				<NuxtLink to="/#faq">{{ formatMessage(messages.faq) }}</NuxtLink>
				<a href="#download">{{ formatMessage(messages.downloads) }}</a>
				<NuxtLink to="/changelog">{{ formatMessage(messages.changelog) }}</NuxtLink>
				<NuxtLink to="/terms">{{ formatMessage(messages.terms) }}</NuxtLink>
				<NuxtLink to="/privacy">{{ formatMessage(messages.privacy) }}</NuxtLink>
			</nav>

			<p class="col-span-full m-0 text-center text-[0.64rem] text-[var(--color-secondary)] tracking-[0.015em] opacity-45">{{ formatMessage(messages.disclaimer) }}</p>
		</div>
	</footer>
</template>

<style scoped lang="scss">
.site-footer {
	&::before {
		position: absolute;
		top: -1px;
		left: 50%;
		width: min(76rem, calc(100% - 3rem));
		height: 1px;
		transform: translateX(-50%);
		background: linear-gradient(90deg, transparent, var(--color-brand), transparent);
		content: '';
		opacity: 0.6;
	}
}

.footer-inner {
	display: grid;
	grid-template-columns: minmax(0, 1fr) auto;
	align-items: center;
	gap: 0.8rem 2.5rem;
	width: min(76rem, calc(100% - 3rem));
	margin: 0 auto;
	padding: 1.75rem 0;
}

.footer-legal {
	p {
		margin: 0;
	}

	a {
		color: var(--color-base);
		text-underline-offset: 0.15rem;
	}
}

.footer-links {
	a {
		color: var(--color-secondary);
		font-size: 0.82rem;
		font-weight: 500;
		text-decoration: none;
		transition: color 140ms ease;

		&:hover {
			color: var(--color-brand);
		}
	}
}

@media (max-width: 900px) {
	.footer-inner {
		grid-template-columns: 1fr;
	}

	.footer-links {
		justify-content: flex-start;
	}
}

@media (max-width: 560px) {
	.footer-inner {
		width: calc(100% - 2rem);
	}
}
</style>
