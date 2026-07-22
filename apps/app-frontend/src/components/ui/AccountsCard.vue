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
		<ButtonStyled v-if="!offline">
			<button :disabled="loginDisabled" @click="showYggdrasilAccountModal()">
				<PlusIcon />
				{{ formatMessage(messages.addThirdPartyAccount) }}
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
								: selectedAccount?.account_type === 'yggdrasil'
									? selectedAccount.yggdrasil?.server_name ||
										formatMessage(messages.thirdPartyAccount)
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
						<span
							v-else-if="account.account_type === 'yggdrasil'"
							class="text-secondary text-xs shrink-0"
						>
							{{ account.yggdrasil?.server_name || formatMessage(messages.thirdPartyBadge) }}
						</span>
					</button>
					<ButtonStyled circular color="red" color-fill="none" hover-color-fill="background">
						<button
							v-tooltip="formatMessage(messages.removeAccount)"
							class="mr-2"
							@click="logout(account)"
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
				<ButtonStyled v-if="accounts.length > 0 && !offline" class="w-full">
					<button :disabled="loginDisabled" @click="showYggdrasilAccountModal()">
						<PlusIcon />
						{{ formatMessage(messages.addThirdPartyAccount) }}
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
	<ModalWrapper ref="yggdrasilAccountModal" :header="formatMessage(messages.thirdPartyModalTitle)">
		<div class="flex min-w-[24rem] flex-col gap-4">
			<p class="m-0 text-secondary">{{ formatMessage(messages.thirdPartyModalDescription) }}</p>
			<div v-if="savedYggdrasilLogins.length > 0" class="flex flex-col gap-2">
				<span class="font-semibold">{{ formatMessage(messages.savedLogins) }}</span>
				<div
					v-for="savedLogin in savedYggdrasilLogins"
					:key="`${savedLogin.api_root}:${savedLogin.login}`"
					class="flex items-center gap-1 rounded-xl bg-surface-3 p-1"
				>
					<button
						class="flex min-w-0 flex-grow flex-col items-start border-0 bg-transparent px-3 py-2 text-left cursor-pointer"
						:disabled="loginDisabled"
						@click="selectSavedYggdrasilLogin(savedLogin)"
					>
						<span class="w-full truncate font-semibold text-primary">{{ savedLogin.login }}</span>
						<span class="w-full truncate text-xs text-secondary">{{ savedLogin.api_root }}</span>
					</button>
					<ButtonStyled circular color="red" color-fill="none" hover-color-fill="background">
						<button
							v-tooltip="formatMessage(messages.removeSavedLogin)"
							:disabled="loginDisabled"
							@click="removeSavedYggdrasilLogin(savedLogin)"
						>
							<TrashIcon />
						</button>
					</ButtonStyled>
				</div>
			</div>
			<ButtonStyled class="w-full">
				<button :disabled="loginDisabled" @click="useLittleSkinPreset()">
					{{ formatMessage(messages.useLittleSkin) }}
				</button>
			</ButtonStyled>
			<label class="flex flex-col gap-2 font-semibold">
				{{ formatMessage(messages.apiRootLabel) }}
				<StyledInput
					v-model="yggdrasilApiRoot"
					:disabled="loginDisabled"
					:placeholder="formatMessage(messages.apiRootPlaceholder)"
					inputmode="url"
					@blur="loadRememberedYggdrasilPassword()"
				/>
			</label>
			<label class="flex flex-col gap-2 font-semibold">
				{{ formatMessage(messages.accountLabel) }}
				<StyledInput
					v-model="yggdrasilLogin"
					:disabled="loginDisabled"
					:placeholder="formatMessage(messages.accountPlaceholder)"
					autocomplete="username"
					@blur="loadRememberedYggdrasilPassword()"
				/>
			</label>
			<label class="flex flex-col gap-2 font-semibold">
				{{ formatMessage(messages.passwordLabel) }}
				<StyledInput
					v-model="yggdrasilPassword"
					type="password"
					:disabled="loginDisabled"
					autocomplete="current-password"
					@keyup.enter="addYggdrasilAccount()"
				/>
			</label>
			<Checkbox
				v-model="rememberYggdrasilPassword"
				:disabled="loginDisabled"
				:label="formatMessage(messages.rememberPassword)"
			/>
			<div class="input-group push-right">
				<ButtonStyled>
					<button :disabled="loginDisabled" @click="yggdrasilAccountModal?.hide()">
						{{ formatMessage(commonMessages.cancelButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button :disabled="loginDisabled || !yggdrasilFormValid" @click="addYggdrasilAccount()">
						<SpinnerIcon v-if="loginDisabled" class="animate-spin" />
						<LogInIcon v-else />
						{{ formatMessage(messages.signInButton) }}
					</button>
				</ButtonStyled>
			</div>
		</div>
	</ModalWrapper>
	<ModalWrapper ref="yggdrasilProfileModal" :header="formatMessage(messages.selectProfileTitle)">
		<div class="flex min-w-[22rem] flex-col gap-2">
			<p class="m-0 mb-2 text-secondary">{{ formatMessage(messages.selectProfileDescription) }}</p>
			<ButtonStyled v-for="profile in pendingYggdrasilProfiles" :key="profile.id" class="w-full">
				<button :disabled="loginDisabled" @click="selectYggdrasilProfile(profile.id)">
					<SpinnerIcon v-if="loginDisabled" class="animate-spin" />
					<RadioButtonIcon v-else />
					{{ profile.name }}
				</button>
			</ButtonStyled>
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
	Checkbox,
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
	begin_yggdrasil_login,
	delete_yggdrasil_password,
	finish_yggdrasil_login,
	get_default_user,
	get_yggdrasil_password,
	list_yggdrasil_saved_logins,
	login as login_flow,
	remove_user,
	set_default_user,
	set_yggdrasil_password,
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
	account_type: 'microsoft' | 'offline' | 'yggdrasil'
	profile: {
		id: string
		name: string
		skins?: Array<{
			state: string
			url: string
			variant: Skin['variant']
			textureKey?: string
		}>
	}
	yggdrasil?: {
		api_root: string
		server_name: string
		login: string
	}
}

type YggdrasilProfile = {
	id: string
	name: string
}

type SavedYggdrasilLogin = {
	api_root: string
	login: string
}

type YggdrasilLoginResult =
	| { status: 'complete'; credentials: MinecraftCredential }
	| { status: 'select_profile'; flow_id: string; profiles: YggdrasilProfile[] }

const LITTLE_SKIN_API_ROOT = 'https://littleskin.cn/api/yggdrasil'

const accounts: Ref<MinecraftCredential[]> = ref([])
const loginDisabled = ref(false)
const defaultUser = ref<string | undefined>()
const equippedSkin = ref<Skin | null>(null)
const headUrlCache = ref(new Map<string, string>())
const accountHeadUrlCache = ref(new Map<string, string>())
let refreshGeneration = 0
let defaultUserUpdateQueue = Promise.resolve()
const offlineAccountModal = ref<InstanceType<typeof ModalWrapper> | null>(null)
const offlineUsername = ref('')
const offlineUsernameValid = computed(() =>
	/^[\p{L}\p{N}_]{1,16}$/u.test(offlineUsername.value.trim()),
)
const yggdrasilAccountModal = ref<InstanceType<typeof ModalWrapper> | null>(null)
const yggdrasilProfileModal = ref<InstanceType<typeof ModalWrapper> | null>(null)
const yggdrasilApiRoot = ref(LITTLE_SKIN_API_ROOT)
const yggdrasilLogin = ref('')
const yggdrasilPassword = ref('')
const rememberYggdrasilPassword = ref(true)
const savedYggdrasilLogins = ref<SavedYggdrasilLogin[]>([])
const pendingYggdrasilFlowId = ref<string | undefined>()
const pendingYggdrasilProfiles = ref<YggdrasilProfile[]>([])
const yggdrasilFormValid = computed(
	() =>
		yggdrasilApiRoot.value.trim().length > 0 &&
		yggdrasilLogin.value.trim().length > 0 &&
		yggdrasilPassword.value.length > 0,
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

const ACCOUNT_TYPE_ORDER = {
	microsoft: 0,
	yggdrasil: 1,
	offline: 2,
} as const

async function refreshValues() {
	const generation = ++refreshGeneration
	const selectedUser = await get_default_user(offline.value).catch(handleError)
	if (generation !== refreshGeneration) return

	defaultUser.value = selectedUser
	if (offline.value && selectedUser) {
		await persistDefaultUser(selectedUser)
		if (generation !== refreshGeneration) return
	}
	const userList = await users(offline.value).catch(handleError)
	if (generation !== refreshGeneration) return
	// The Rust backend returns a plain array that structurally matches
	// MinecraftCredential but the TS types from the Tauri IPC bridge do not
	// carry this refinement. The double cast is deliberate — the shape is
	// correct and verified at runtime by the backend.
	accounts.value = Array.isArray(userList)
		? [...(userList as unknown as MinecraftCredential[])]
		: []
	accounts.value.sort((a, b) => {
		const nameCmp = (a.profile?.name ?? '').localeCompare(b.profile?.name ?? '')
		if (nameCmp !== 0) return nameCmp

		return (
			(ACCOUNT_TYPE_ORDER[a.account_type as keyof typeof ACCOUNT_TYPE_ORDER] ?? 3) -
			(ACCOUNT_TYPE_ORDER[b.account_type as keyof typeof ACCOUNT_TYPE_ORDER] ?? 3)
		)
	})
	await renderAccountHeads(accounts.value)
	if (generation !== refreshGeneration) return
	try {
		const skins = await get_available_skins()
		if (generation !== refreshGeneration) return
		equippedSkin.value = skins.find((skin) => skin.is_equipped) ?? null

		if (equippedSkin.value) {
			try {
				const headUrl = await getPlayerHeadUrl(equippedSkin.value)
				if (generation !== refreshGeneration) return
				headUrlCache.value = new Map(headUrlCache.value).set(
					equippedSkin.value.texture_key,
					headUrl,
				)
				if (selectedUser) {
					accountHeadUrlCache.value = new Map(accountHeadUrlCache.value).set(
						selectedUser,
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
	const selectedUser = defaultUser.value
	equippedSkin.value = skin

	try {
		const headUrl = await getPlayerHeadUrl(skin)
		headUrlCache.value = new Map(headUrlCache.value).set(skin.texture_key, headUrl)
		if (selectedUser) {
			accountHeadUrlCache.value = new Map(accountHeadUrlCache.value).set(selectedUser, headUrl)
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

function getAccountSkin(account: MinecraftCredential | undefined): Skin | undefined {
	if (!account || account.account_type === 'offline') return undefined
	const skin =
		account.profile.skins?.find((skin) => skin.state === 'ACTIVE') ?? account.profile.skins?.[0]
	if (!skin?.url) return undefined

	return {
		texture_key: skin.textureKey ?? `${account.profile.id}:${skin.url}`,
		variant: skin.variant ?? 'UNKNOWN',
		texture: skin.url,
		source: 'custom_external',
		is_equipped: true,
	}
}

async function renderAccountHeads(accountList: MinecraftCredential[]) {
	await Promise.all(
		accountList.map(async (account) => {
			const skin = getAccountSkin(account)
			if (!skin) return

			try {
				const headUrl = await getPlayerHeadUrl(skin)
				accountHeadUrlCache.value = new Map(accountHeadUrlCache.value).set(
					account.profile.id,
					headUrl,
				)
			} catch (error) {
				console.warn(`Failed to render head for account ${account.profile.id}:`, error)
			}
		}),
	)
}

const avatarUrl = computed(() => {
	if (selectedAccount.value) {
		const cachedHeadUrl = accountHeadUrlCache.value.get(selectedAccount.value.profile.id)
		if (cachedHeadUrl) return cachedHeadUrl
	}
	if (equippedSkin.value?.texture_key) {
		const cachedUrl = headUrlCache.value.get(equippedSkin.value.texture_key)
		if (cachedUrl) {
			return cachedUrl
		}
	}
	return selectedAccount.value ? defaultSteveHeadUrl : axolotlLogo
})

function getAccountAvatarUrl(account: MinecraftCredential) {
	const cachedHeadUrl = accountHeadUrlCache.value.get(account.profile.id)
	if (cachedHeadUrl) {
		return cachedHeadUrl
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
	return defaultSteveHeadUrl
}

function persistDefaultUser(userId: string) {
	const update = defaultUserUpdateQueue.then(async () => {
		await set_default_user(userId).catch(handleError)
	})
	defaultUserUpdateQueue = update.catch(() => {})
	return update
}

async function setAccount(account: MinecraftCredential) {
	const userId = account.profile.id
	refreshGeneration += 1
	defaultUser.value = userId
	equippedSkin.value = null

	await persistDefaultUser(userId)
	if (defaultUser.value !== userId) return
	await refreshValues()
	if (defaultUser.value === userId) emit('change')
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

async function showYggdrasilAccountModal() {
	yggdrasilApiRoot.value = LITTLE_SKIN_API_ROOT
	yggdrasilLogin.value = ''
	yggdrasilPassword.value = ''
	rememberYggdrasilPassword.value = true
	pendingYggdrasilFlowId.value = undefined
	pendingYggdrasilProfiles.value = []
	await loadSavedYggdrasilLogins()
	yggdrasilAccountModal.value?.show()
}

async function loadSavedYggdrasilLogins() {
	const storedLogins = await list_yggdrasil_saved_logins().catch(handleError)
	const savedLogins: SavedYggdrasilLogin[] = Array.isArray(storedLogins) ? [...storedLogins] : []
	const savedLoginKeys = new Set(
		savedLogins.map((savedLogin) => `${savedLogin.api_root}\n${savedLogin.login}`),
	)

	for (const account of accounts.value) {
		if (!account.yggdrasil) continue
		const savedLogin = {
			api_root: account.yggdrasil.api_root,
			login: account.yggdrasil.login,
		}
		const key = `${savedLogin.api_root}\n${savedLogin.login}`
		if (savedLoginKeys.has(key)) continue

		try {
			const password = await get_yggdrasil_password(savedLogin.api_root, savedLogin.login)
			if (!password) continue
			await set_yggdrasil_password(savedLogin.api_root, savedLogin.login, password)
			savedLogins.push(savedLogin)
			savedLoginKeys.add(key)
		} catch {
			continue
		}
	}

	savedYggdrasilLogins.value = savedLogins.sort((left, right) =>
		left.login.localeCompare(right.login),
	)
}

async function selectSavedYggdrasilLogin(savedLogin: SavedYggdrasilLogin) {
	if (loginDisabled.value) return

	loginDisabled.value = true
	try {
		const password = await get_yggdrasil_password(savedLogin.api_root, savedLogin.login)
		if (!password) {
			await delete_yggdrasil_password(savedLogin.api_root, savedLogin.login)
			await loadSavedYggdrasilLogins()
			return
		}
		yggdrasilApiRoot.value = savedLogin.api_root
		yggdrasilLogin.value = savedLogin.login
		yggdrasilPassword.value = password
		rememberYggdrasilPassword.value = true
	} catch (error) {
		handleError(error as Error)
	} finally {
		loginDisabled.value = false
	}
}

async function removeSavedYggdrasilLogin(savedLogin: SavedYggdrasilLogin) {
	if (loginDisabled.value) return

	loginDisabled.value = true
	try {
		await delete_yggdrasil_password(savedLogin.api_root, savedLogin.login)
		savedYggdrasilLogins.value = savedYggdrasilLogins.value.filter(
			(entry) => entry.api_root !== savedLogin.api_root || entry.login !== savedLogin.login,
		)
		if (
			yggdrasilApiRoot.value === savedLogin.api_root &&
			yggdrasilLogin.value === savedLogin.login
		) {
			yggdrasilPassword.value = ''
			rememberYggdrasilPassword.value = false
		}
	} catch (error) {
		handleError(error as Error)
	} finally {
		loginDisabled.value = false
	}
}

function useLittleSkinPreset() {
	yggdrasilApiRoot.value = LITTLE_SKIN_API_ROOT
}

async function loadRememberedYggdrasilPassword() {
	if (
		!rememberYggdrasilPassword.value ||
		!yggdrasilApiRoot.value.trim() ||
		!yggdrasilLogin.value.trim() ||
		yggdrasilPassword.value
	)
		return

	try {
		const password = await get_yggdrasil_password(
			yggdrasilApiRoot.value.trim(),
			yggdrasilLogin.value.trim(),
		)
		if (password) yggdrasilPassword.value = password
	} catch {
		return
	}
}

async function persistYggdrasilPasswordPreference() {
	try {
		if (rememberYggdrasilPassword.value) {
			await set_yggdrasil_password(
				yggdrasilApiRoot.value.trim(),
				yggdrasilLogin.value.trim(),
				yggdrasilPassword.value,
			)
		} else {
			await delete_yggdrasil_password(yggdrasilApiRoot.value.trim(), yggdrasilLogin.value.trim())
		}
	} catch (error) {
		handleError(error as Error)
	}
}

async function addYggdrasilAccount() {
	if (!yggdrasilFormValid.value || loginDisabled.value) return

	loginDisabled.value = true
	try {
		const result = (await begin_yggdrasil_login(
			yggdrasilApiRoot.value.trim(),
			yggdrasilLogin.value.trim(),
			yggdrasilPassword.value,
		)) as YggdrasilLoginResult
		if (result.status === 'complete') {
			await persistYggdrasilPasswordPreference()
			yggdrasilAccountModal.value?.hide()
			await setAccount(result.credentials)
			trackEvent('YggdrasilAccountAdd')
		} else {
			pendingYggdrasilFlowId.value = result.flow_id
			pendingYggdrasilProfiles.value = result.profiles
			yggdrasilAccountModal.value?.hide()
			yggdrasilProfileModal.value?.show()
		}
	} catch (error) {
		handleError(error as Error)
	} finally {
		loginDisabled.value = false
	}
}

async function selectYggdrasilProfile(profileId: string) {
	if (!pendingYggdrasilFlowId.value || loginDisabled.value) return

	loginDisabled.value = true
	try {
		const account = (await finish_yggdrasil_login(
			pendingYggdrasilFlowId.value,
			profileId,
		)) as MinecraftCredential
		await persistYggdrasilPasswordPreference()
		yggdrasilProfileModal.value?.hide()
		await setAccount(account)
		trackEvent('YggdrasilAccountAdd')
	} catch (error) {
		handleError(error as Error)
	} finally {
		loginDisabled.value = false
	}
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

async function logout(account: MinecraftCredential) {
	await remove_user(account.profile.id).catch(handleError)
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
	addThirdPartyAccount: {
		id: 'minecraft-account.add-third-party-account',
		defaultMessage: 'Add third-party account',
	},
	thirdPartyAccount: {
		id: 'minecraft-account.third-party-account',
		defaultMessage: 'Third-party Minecraft account',
	},
	thirdPartyBadge: {
		id: 'minecraft-account.third-party-badge',
		defaultMessage: 'Third-party',
	},
	thirdPartyModalTitle: {
		id: 'minecraft-account.third-party-modal.title',
		defaultMessage: 'Sign in with a third-party service',
	},
	thirdPartyModalDescription: {
		id: 'minecraft-account.third-party-modal.description',
		defaultMessage: 'Use LittleSkin or another compatible Yggdrasil authentication service.',
	},
	useLittleSkin: {
		id: 'minecraft-account.third-party-modal.littleskin',
		defaultMessage: 'Use LittleSkin',
	},
	apiRootLabel: {
		id: 'minecraft-account.third-party-modal.api-root',
		defaultMessage: 'Yggdrasil API address',
	},
	apiRootPlaceholder: {
		id: 'minecraft-account.third-party-modal.api-root-placeholder',
		defaultMessage: 'https://example.com/api/yggdrasil',
	},
	accountLabel: {
		id: 'minecraft-account.third-party-modal.account',
		defaultMessage: 'Account or email',
	},
	accountPlaceholder: {
		id: 'minecraft-account.third-party-modal.account-placeholder',
		defaultMessage: 'Enter your account or email',
	},
	passwordLabel: {
		id: 'minecraft-account.third-party-modal.password',
		defaultMessage: 'Password',
	},
	rememberPassword: {
		id: 'minecraft-account.third-party-modal.remember-password',
		defaultMessage: 'Save this login on this device',
	},
	savedLogins: {
		id: 'minecraft-account.third-party-modal.saved-logins',
		defaultMessage: 'Saved logins',
	},
	removeSavedLogin: {
		id: 'minecraft-account.third-party-modal.remove-saved-login',
		defaultMessage: 'Remove saved login',
	},
	signInButton: {
		id: 'minecraft-account.third-party-modal.sign-in',
		defaultMessage: 'Sign in',
	},
	selectProfileTitle: {
		id: 'minecraft-account.third-party-profile.title',
		defaultMessage: 'Select a profile',
	},
	selectProfileDescription: {
		id: 'minecraft-account.third-party-profile.description',
		defaultMessage: 'Choose the Minecraft profile to use with this account.',
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
		defaultMessage: 'Use 1–16 letters, numbers, or underscores, including Chinese characters.',
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
