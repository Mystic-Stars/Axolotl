<template>
	<div
		v-if="offline"
		class="flex flex-col gap-1 bg-highlight-orange border border-solid border-orange rounded-xl p-3 mt-2"
	>
		<span class="font-semibold text-contrast">{{ formatMessage(messages.offlineMode) }}</span>
		<span class="text-sm text-secondary">{{ formatMessage(messages.offlineModeDescription) }}</span>
	</div>
	<div
		v-if="accounts.length === 0"
		class="flex flex-col gap-3 bg-button-bg border border-solid border-surface-5 rounded-xl p-3 mt-2"
	>
		<span>{{ formatMessage(messages.notSignedIn) }}</span>
		<ButtonStyled v-if="!offline" color="brand">
			<button color="primary" :disabled="loginDisabled" @click="login()">
				<LogInIcon v-if="!loginDisabled" />
				<SpinnerIcon v-else class="animate-spin" />
				{{ formatMessage(messages.signInToMinecraft) }}
			</button>
		</ButtonStyled>
		<ButtonStyled>
			<button :disabled="loginDisabled" @click="showOfflineAccountModal()">
				<PlusIcon />
				{{ formatMessage(messages.addOfflineAccount) }}
			</button>
		</ButtonStyled>
	</div>
	<Accordion
		v-else
		class="w-full mt-2 bg-button-bg border border-solid border-surface-5 rounded-xl overflow-clip"
		button-class="button-base w-full bg-transparent px-3 py-2 border-0 cursor-pointer"
		:open-by-default="false"
	>
		<template #title>
			<div class="flex gap-2 w-full min-w-0">
				<Avatar size="36px" :src="selectedAccount ? avatarUrl : axolotlLogo" />
				<div class="flex flex-col items-start w-full min-w-0">
					<span class="truncate w-full text-left">{{
						selectedAccount ? selectedAccount.profile.name : formatMessage(messages.selectAccount)
					}}</span>
					<span class="text-secondary text-xs">
						{{
							selectedAccount?.account_type === 'offline'
								? formatMessage(messages.offlineAccount)
								: formatMessage(messages.minecraftAccount)
						}}
					</span>
				</div>
			</div>
		</template>
		<div class="bg-button-bg pt-1 pb-2 border border-solid border-surface-5">
			<template v-if="accounts.length > 0">
				<div v-for="account in accounts" :key="account.profile.id" class="flex gap-1 items-center">
					<button
						class="flex items-center flex-shrink flex-grow overflow-clip gap-2 p-2 border-0 bg-transparent cursor-pointer button-base min-w-0"
						@click="setAccount(account)"
					>
						<RadioButtonCheckedIcon
							v-if="selectedAccount && selectedAccount.profile.id === account.profile.id"
							class="w-5 h-5 text-brand shrink-0"
						/>
						<RadioButtonIcon v-else class="w-5 h-5 text-secondary shrink-0" />
						<Avatar :src="getAccountAvatarUrl(account)" size="24px" />
						<p
							class="m-0 truncate min-w-0"
							:class="
								selectedAccount && selectedAccount.profile.id === account.profile.id
									? 'text-contrast font-semibold'
									: 'text-primary'
							"
						>
							{{ account.profile.name }}
						</p>
						<span v-if="account.account_type === 'offline'" class="text-secondary text-xs shrink-0">
							{{ formatMessage(messages.offlineBadge) }}
						</span>
					</button>
					<ButtonStyled circular color="red" color-fill="none" hover-color-fill="background">
						<button
							v-tooltip="formatMessage(messages.removeAccount)"
							class="mr-2"
							@click="logout(account.profile.id)"
						>
							<TrashIcon />
						</button>
					</ButtonStyled>
				</div>
			</template>
			<div class="flex flex-col gap-2 px-2 pt-2">
				<ButtonStyled v-if="accounts.length > 0 && !offline" class="w-full">
					<button :disabled="loginDisabled" @click="login()">
						<PlusIcon />
						{{ formatMessage(messages.addMicrosoftAccount) }}
					</button>
				</ButtonStyled>
				<ButtonStyled v-if="accounts.length > 0" class="w-full">
					<button :disabled="loginDisabled" @click="showOfflineAccountModal()">
						<PlusIcon />
						{{ formatMessage(messages.addOfflineAccount) }}
					</button>
				</ButtonStyled>
			</div>
		</div>
	</Accordion>
	<ModalWrapper ref="offlineAccountModal" :header="formatMessage(messages.offlineModalTitle)">
		<div class="flex min-w-[22rem] flex-col gap-4">
			<p class="m-0 text-secondary">{{ formatMessage(messages.offlineModalDescription) }}</p>
			<label class="flex flex-col gap-2 font-semibold">
				{{ formatMessage(messages.usernameLabel) }}
				<StyledInput
					v-model="offlineUsername"
					:disabled="loginDisabled"
					:placeholder="formatMessage(messages.usernamePlaceholder)"
					autocomplete="off"
					maxlength="16"
					@keyup.enter="addOfflineAccount()"
				/>
			</label>
			<p v-if="offlineUsername.length > 0 && !offlineUsernameValid" class="m-0 text-sm text-red">
				{{ formatMessage(messages.usernameValidation) }}
			</p>
			<div class="input-group push-right">
				<ButtonStyled>
					<button :disabled="loginDisabled" @click="offlineAccountModal?.hide()">
						{{ formatMessage(commonMessages.cancelButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button :disabled="loginDisabled || !offlineUsernameValid" @click="addOfflineAccount()">
						<SpinnerIcon v-if="loginDisabled" class="animate-spin" />
						<PlusIcon v-else />
						{{ formatMessage(messages.createOfflineAccount) }}
					</button>
				</ButtonStyled>
			</div>
		</div>
	</ModalWrapper>
</template>

<script setup lang="ts">
import {
	LogInIcon,
	PlusIcon,
	RadioButtonCheckedIcon,
	RadioButtonIcon,
	SpinnerIcon,
	TrashIcon,
} from '@modrinth/assets'
import {
	Accordion,
	Avatar,
	ButtonStyled,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import type { Ref } from 'vue'
import { computed, onUnmounted, ref, watch } from 'vue'

import axolotlLogo from '@/assets/axolotl.png'
import steveSkinTexture from '@/assets/skins/steve.png'
import ModalWrapper from '@/components/ui/modal/ModalWrapper.vue'
import { useNetworkStatus } from '@/composables/useNetworkStatus'
import { trackEvent } from '@/helpers/analytics'
import {
	add_offline_user,
	get_default_user,
	login as login_flow,
	remove_user,
	set_default_user,
	users,
} from '@/helpers/auth'
import { process_listener } from '@/helpers/events'
import { getPlayerHeadUrl } from '@/helpers/rendering/batch-skin-renderer.ts'
import type { Skin } from '@/helpers/skins'
import { get_available_skins } from '@/helpers/skins'
import { handleSevereError } from '@/store/error.js'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const { offline } = useNetworkStatus()

const emit = defineEmits<{
	change: []
}>()

type MinecraftCredential = {
	account_type: 'microsoft' | 'offline'
	profile: {
		id: string
		name: string
	}
}

const accounts: Ref<MinecraftCredential[]> = ref([])
const loginDisabled = ref(false)
const defaultUser = ref<string | undefined>()
const equippedSkin = ref<Skin | null>(null)
const headUrlCache = ref(new Map<string, string>())
const accountHeadUrlCache = ref(new Map<string, string>())
const offlineAccountModal = ref<InstanceType<typeof ModalWrapper> | null>(null)
const offlineUsername = ref('')
const offlineUsernameValid = computed(() =>
	/^[A-Za-z0-9_]{3,16}$/.test(offlineUsername.value.trim()),
)

function createSkinHeadDataUrl(textureUrl: string) {
	const escapedTextureUrl = textureUrl
		.replaceAll('&', '&amp;')
		.replaceAll('"', '&quot;')
		.replaceAll('<', '&lt;')
		.replaceAll('>', '&gt;')
	const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 8 8" shape-rendering="crispEdges"><image href="${escapedTextureUrl}" x="-8" y="-8" width="64" height="64" style="image-rendering:pixelated"/><image href="${escapedTextureUrl}" x="-40" y="-8" width="64" height="64" style="image-rendering:pixelated"/></svg>`

	return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`
}

const defaultSteveHeadUrl = createSkinHeadDataUrl(steveSkinTexture)

async function refreshValues() {
	defaultUser.value = await get_default_user(offline.value).catch(handleError)
	if (offline.value && defaultUser.value) {
		await set_default_user(defaultUser.value).catch(handleError)
	}
	const userList = await users(offline.value).catch(handleError)
	accounts.value = Array.isArray(userList) ? [...userList] : []
	accounts.value.sort((a, b) => (a.profile?.name ?? '').localeCompare(b.profile?.name ?? ''))
	try {
		const skins = await get_available_skins()
		equippedSkin.value = skins.find((skin) => skin.is_equipped) ?? null

		if (equippedSkin.value) {
			try {
				const headUrl = await getPlayerHeadUrl(equippedSkin.value)
				headUrlCache.value = new Map(headUrlCache.value).set(
					equippedSkin.value.texture_key,
					headUrl,
				)
				if (defaultUser.value) {
					accountHeadUrlCache.value = new Map(accountHeadUrlCache.value).set(
						defaultUser.value,
						headUrl,
					)
				}
			} catch (error) {
				console.warn('Failed to get head render for equipped skin:', error)
			}
		}
	} catch {
		equippedSkin.value = null
	}
}

async function setEquippedSkin(skin: Skin) {
	equippedSkin.value = skin

	try {
		const headUrl = await getPlayerHeadUrl(skin)
		headUrlCache.value = new Map(headUrlCache.value).set(skin.texture_key, headUrl)
		if (defaultUser.value) {
			accountHeadUrlCache.value = new Map(accountHeadUrlCache.value).set(defaultUser.value, headUrl)
		}
	} catch (error) {
		console.warn('Failed to get head render for equipped skin:', error)
	}
}

function setLoginDisabled(value: boolean) {
	loginDisabled.value = value
}

defineExpose({
	refreshValues,
	setEquippedSkin,
	setLoginDisabled,
	loginDisabled,
})

await refreshValues()

watch(offline, async () => {
	await refreshValues()
	emit('change')
})

const selectedAccount = computed(() =>
	accounts.value.find((account) => account.profile.id === defaultUser.value),
)

const avatarUrl = computed(() => {
	if (equippedSkin.value?.texture_key) {
		const cachedUrl = headUrlCache.value.get(equippedSkin.value.texture_key)
		if (cachedUrl) {
			return cachedUrl
		}
		if (selectedAccount.value?.account_type === 'offline') {
			return defaultSteveHeadUrl
		}
		return `https://mc-heads.net/avatar/${equippedSkin.value.texture_key}/128`
	}
	if (selectedAccount.value?.account_type === 'offline') {
		return defaultSteveHeadUrl
	}
	if (selectedAccount.value?.profile?.id) {
		return `https://mc-heads.net/avatar/${selectedAccount.value.profile.id}/128`
	}
	return axolotlLogo
})

function getAccountAvatarUrl(account: MinecraftCredential) {
	if (account.account_type === 'offline') {
		return accountHeadUrlCache.value.get(account.profile.id) ?? defaultSteveHeadUrl
	}
	if (
		account.profile.id === selectedAccount.value?.profile?.id &&
		equippedSkin.value?.texture_key
	) {
		const cachedUrl = headUrlCache.value.get(equippedSkin.value.texture_key)
		if (cachedUrl) {
			return cachedUrl
		}
	}
	return `https://mc-heads.net/avatar/${account.profile.id}/128`
}

async function setAccount(account: MinecraftCredential) {
	defaultUser.value = account.profile.id
	await set_default_user(account.profile.id).catch(handleError)
	await refreshValues()
	emit('change')
}

async function login() {
	if (offline.value) return

	loginDisabled.value = true
	const loggedIn = await login_flow().catch(handleSevereError)

	if (loggedIn) {
		await setAccount(loggedIn)
	}

	trackEvent('AccountLogIn')
	loginDisabled.value = false
}

function showOfflineAccountModal() {
	offlineUsername.value = ''
	offlineAccountModal.value?.show()
}

async function addOfflineAccount() {
	if (!offlineUsernameValid.value || loginDisabled.value) return

	loginDisabled.value = true
	try {
		const account = await add_offline_user(offlineUsername.value.trim())
		offlineAccountModal.value?.hide()
		await setAccount(account)
		trackEvent('OfflineAccountAdd')
	} catch (error) {
		handleError(error as Error)
	} finally {
		loginDisabled.value = false
	}
}

async function logout(id: string) {
	await remove_user(id).catch(handleError)
	await refreshValues()
	if (!selectedAccount.value && accounts.value.length > 0) {
		await setAccount(accounts.value[0])
	} else {
		emit('change')
	}
	trackEvent('AccountLogOut')
}

const unlisten = await process_listener(async (e) => {
	if (e.event === 'launched') {
		await refreshValues()
	}
})

onUnmounted(() => {
	unlisten()
})

const messages = defineMessages({
	offlineMode: {
		id: 'minecraft-account.offline-mode',
		defaultMessage: 'Offline mode',
	},
	offlineModeDescription: {
		id: 'minecraft-account.offline-mode.description',
		defaultMessage:
			'Only offline accounts are available. You can launch fully downloaded instances.',
	},
	notSignedIn: {
		id: 'minecraft-account.not-signed-in',
		defaultMessage: 'Not signed in',
	},
	addMicrosoftAccount: {
		id: 'minecraft-account.add-microsoft-account',
		defaultMessage: 'Add Microsoft account',
	},
	addOfflineAccount: {
		id: 'minecraft-account.add-offline-account',
		defaultMessage: 'Add offline account',
	},
	offlineAccount: {
		id: 'minecraft-account.offline-account',
		defaultMessage: 'Offline Minecraft account',
	},
	offlineBadge: {
		id: 'minecraft-account.offline-badge',
		defaultMessage: 'Offline',
	},
	offlineModalTitle: {
		id: 'minecraft-account.offline-modal.title',
		defaultMessage: 'Add offline account',
	},
	offlineModalDescription: {
		id: 'minecraft-account.offline-modal.description',
		defaultMessage:
			'Choose the username used in offline games. This account can only join servers that allow offline players.',
	},
	usernameLabel: {
		id: 'minecraft-account.offline-modal.username-label',
		defaultMessage: 'Minecraft username',
	},
	usernamePlaceholder: {
		id: 'minecraft-account.offline-modal.username-placeholder',
		defaultMessage: 'Enter a username',
	},
	usernameValidation: {
		id: 'minecraft-account.offline-modal.username-validation',
		defaultMessage: 'Use 3–16 letters, numbers, or underscores.',
	},
	createOfflineAccount: {
		id: 'minecraft-account.offline-modal.create',
		defaultMessage: 'Create account',
	},
	removeAccount: {
		id: 'minecraft-account.remove-account',
		defaultMessage: 'Remove account',
	},
	selectAccount: {
		id: 'minecraft-account.select-account',
		defaultMessage: 'Select account',
	},
	minecraftAccount: {
		id: 'minecraft-account.label',
		defaultMessage: 'Minecraft account',
	},
	signInToMinecraft: {
		id: 'minecraft-account.sign-in',
		defaultMessage: 'Sign in to Minecraft',
	},
})
</script>
