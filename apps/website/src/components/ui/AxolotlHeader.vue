<script setup lang="ts">
import GithubIcon from '@modrinth/assets/external/github.svg?component'
import DownloadIcon from '@modrinth/assets/icons/download.svg?component'
import HamburgerIcon from '@modrinth/assets/icons/hamburger.svg?component'
import SettingsIcon from '@modrinth/assets/icons/settings.svg?component'
import XIcon from '@modrinth/assets/icons/x.svg?component'
import ButtonStyled from '@modrinth/ui/src/components/base/ButtonStyled.vue'
import { defineMessages, useVIntl } from '@modrinth/ui/src/composables/i18n.ts'
import type { ComponentPublicInstance } from 'vue'

import AxolotlWordmark from '~/components/brand/AxolotlWordmark.vue'

const emit = defineEmits<{
	openSettings: []
}>()

const mobileMenuOpen = ref(false)
const mobileMenuRef = ref<HTMLElement | null>(null)
const mobileMenuButtonRef = ref<ComponentPublicInstance | null>(null)
const { formatMessage } = useVIntl()

// 点击菜单与开关之外的区域时收起移动端菜单
function handleOutsideClick(event: MouseEvent) {
	if (!mobileMenuOpen.value) return

	const target = event.target as Node
	const buttonElement = mobileMenuButtonRef.value?.$el
	if (!mobileMenuRef.value?.contains(target) && !buttonElement?.contains(target)) {
		mobileMenuOpen.value = false
	}
}

onMounted(() => {
	document.addEventListener('click', handleOutsideClick)
})

onBeforeUnmount(() => {
	document.removeEventListener('click', handleOutsideClick)
})

const messages = defineMessages({
	home: { id: 'axolotl-site.navigation.home', defaultMessage: 'Axolotl Launcher home' },
	primary: { id: 'axolotl-site.navigation.primary', defaultMessage: 'Primary navigation' },
	mobile: { id: 'axolotl-site.navigation.mobile', defaultMessage: 'Mobile navigation' },
	features: { id: 'axolotl-site.navigation.features', defaultMessage: 'Features' },
	faq: { id: 'axolotl-site.navigation.faq', defaultMessage: 'FAQ' },
	changelog: { id: 'axolotl-site.navigation.changelog', defaultMessage: 'Changelog' },
	terms: { id: 'axolotl-site.navigation.terms', defaultMessage: 'Terms of Service' },
	privacy: { id: 'axolotl-site.navigation.privacy', defaultMessage: 'Privacy Policy' },
	openSource: { id: 'axolotl-site.navigation.open-source', defaultMessage: 'Open source' },
	download: { id: 'axolotl-site.navigation.download', defaultMessage: 'Download' },
	openSettings: {
		id: 'axolotl-site.navigation.open-settings',
		defaultMessage: 'Open display settings',
	},
	openMenu: { id: 'axolotl-site.navigation.open-menu', defaultMessage: 'Open navigation' },
	closeMenu: { id: 'axolotl-site.navigation.close-menu', defaultMessage: 'Close navigation' },
})

function openSettings() {
	mobileMenuOpen.value = false
	emit('openSettings')
}
</script>

<template>
	<header class="absolute top-0 left-0 w-full z-40 pointer-events-none">
		<div class="header-inner">
			<NuxtLink
				to="/"
				:aria-label="formatMessage(messages.home)"
				class="w-fit no-underline button-animation"
			>
				<AxolotlWordmark />
			</NuxtLink>

			<nav
				class="desktop-navigation hidden lg:flex items-center gap-1 pointer-events-auto"
				:aria-label="formatMessage(messages.primary)"
			>
				<ButtonStyled type="transparent">
					<NuxtLink to="/#features">{{ formatMessage(messages.features) }}</NuxtLink>
				</ButtonStyled>
				<ButtonStyled type="transparent">
					<NuxtLink to="/#faq">{{ formatMessage(messages.faq) }}</NuxtLink>
				</ButtonStyled>
				<ButtonStyled type="transparent">
					<NuxtLink to="/changelog">{{ formatMessage(messages.changelog) }}</NuxtLink>
				</ButtonStyled>
				<ButtonStyled type="transparent">
					<NuxtLink to="/terms">{{ formatMessage(messages.terms) }}</NuxtLink>
				</ButtonStyled>
				<ButtonStyled type="transparent">
					<NuxtLink to="/privacy">{{ formatMessage(messages.privacy) }}</NuxtLink>
				</ButtonStyled>
				<ButtonStyled type="transparent">
					<a href="https://github.com/Mystic-Stars/Axolotl" target="_blank" rel="noopener">
						<GithubIcon aria-hidden="true" />
						{{ formatMessage(messages.openSource) }}
					</a>
				</ButtonStyled>
			</nav>

			<div class="header-actions flex items-center gap-1 pointer-events-auto">
				<ButtonStyled class="desktop-download hidden lg:flex" color="brand">
					<a href="#download">
						<DownloadIcon aria-hidden="true" />
						{{ formatMessage(messages.download) }}
					</a>
				</ButtonStyled>
				<ButtonStyled circular type="transparent">
					<button :aria-label="formatMessage(messages.openSettings)" @click="openSettings">
						<SettingsIcon aria-hidden="true" />
					</button>
				</ButtonStyled>
				<ButtonStyled
					ref="mobileMenuButtonRef"
					class="hidden max-lg:flex"
					circular
					type="transparent"
				>
					<button
						:aria-label="formatMessage(mobileMenuOpen ? messages.closeMenu : messages.openMenu)"
						:aria-expanded="mobileMenuOpen"
						@click="mobileMenuOpen = !mobileMenuOpen"
					>
						<XIcon v-if="mobileMenuOpen" aria-hidden="true" />
						<HamburgerIcon v-else aria-hidden="true" />
					</button>
				</ButtonStyled>
			</div>
		</div>

		<Transition name="mobile-menu">
			<nav
				v-if="mobileMenuOpen"
				ref="mobileMenuRef"
				class="mobile-navigation hidden max-lg:flex"
				:aria-label="formatMessage(messages.mobile)"
			>
				<NuxtLink to="/#features" @click="mobileMenuOpen = false">
					{{ formatMessage(messages.features) }}
				</NuxtLink>
				<NuxtLink to="/#faq" @click="mobileMenuOpen = false">
					{{ formatMessage(messages.faq) }}
				</NuxtLink>
				<NuxtLink to="/changelog" @click="mobileMenuOpen = false">
					{{ formatMessage(messages.changelog) }}
				</NuxtLink>
				<NuxtLink to="/terms" @click="mobileMenuOpen = false">
					{{ formatMessage(messages.terms) }}
				</NuxtLink>
				<NuxtLink to="/privacy" @click="mobileMenuOpen = false">
					{{ formatMessage(messages.privacy) }}
				</NuxtLink>
				<a
					href="https://github.com/Mystic-Stars/Axolotl"
					target="_blank"
					rel="noopener"
					@click="mobileMenuOpen = false"
				>
					{{ formatMessage(messages.openSource) }}
				</a>
				<a class="mobile-download" href="#download" @click="mobileMenuOpen = false">
					<DownloadIcon aria-hidden="true" />
					{{ formatMessage(messages.download) }}
				</a>
			</nav>
		</Transition>
	</header>
</template>

<style scoped lang="scss">
.header-inner {
	display: grid;
	grid-template-columns: 1fr auto;
	align-items: center;
	gap: 0.5rem;
	max-width: 1360px;
	margin: 0 auto;
	padding: 1.15rem 1.5rem;
	pointer-events: auto;
}

.desktop-navigation {
	grid-column: 1 / -1;
	grid-row: 2;
	justify-content: center;

	:deep(.button) {
		min-height: 2.25rem;
		border-radius: 0;
		color: var(--color-secondary);
		font-size: 0.78rem;
		font-weight: 700;
		letter-spacing: 0.04em;
		text-transform: uppercase;

		&:hover {
			color: var(--color-contrast);
		}
	}
}

.header-actions {
	grid-column: 2;
	grid-row: 1;
	justify-content: flex-end;
}

.mobile-navigation {
	position: absolute;
	top: 100%;
	right: 1rem;
	width: min(22rem, calc(100% - 2rem));
	flex-direction: column;
	gap: 0.25rem;
	padding: 0.75rem;
	border: 1px solid var(--color-divider);
	border-radius: 1rem;
	background: color-mix(in srgb, var(--color-raised-bg) 92%, transparent);
	box-shadow: 0 1.25rem 3rem rgb(0 0 0 / 22%);
	backdrop-filter: blur(20px) saturate(150%);

	a {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 0.875rem;
		border-radius: var(--radius-md);
		color: var(--color-base);
		font-weight: 600;
		text-decoration: none;

		&:hover {
			background: var(--color-button-bg);
		}
	}

	.mobile-download {
		justify-content: center;
		margin-top: 0.25rem;
		background: var(--color-brand);
		color: var(--color-on-brand);
	}
}

.mobile-menu-enter-active,
.mobile-menu-leave-active {
	transition: 160ms ease;
}

.mobile-menu-enter-from,
.mobile-menu-leave-to {
	transform: translateY(-0.5rem) scale(0.98);
	opacity: 0;
}

@media (min-width: 1024px) {
	.header-inner {
		grid-template-columns: auto 1fr auto;
		padding-top: 1.25rem;
	}

	.desktop-navigation {
		grid-column: 2;
		grid-row: 1;
	}

	.header-actions {
		grid-column: 3;
	}
}

@media (max-width: 1023px) {
	.header-inner {
		grid-template-columns: 1fr auto;
		padding: 0.875rem 1rem;
	}
}
</style>
