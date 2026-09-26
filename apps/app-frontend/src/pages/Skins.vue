<script setup lang="ts">
import {
	CheckIcon,
	EditIcon,
	EyeIcon,
	LogInIcon,
	PlusIcon,
	RotateCounterClockwiseIcon,
	SpinnerIcon,
} from '@modrinth/assets'
import {
	type ArmorPreviewConfig,
	Button,
	commonMessages,
	ConfirmModal,
	defineMessages,
	injectNotificationManager,
	NavTabs,
	SkinPreviewRenderer,
	useVIntl,
} from '@modrinth/ui'
import { arrayBufferToBase64 } from '@modrinth/utils'
import { useQuery } from '@tanstack/vue-query'
import { invoke } from '@tauri-apps/api/core'
import type { DragDropEvent } from '@tauri-apps/api/webview'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { computedAsync } from '@vueuse/core'
import type { Ref } from 'vue'
import { computed, inject, onMounted, onUnmounted, ref, useTemplateRef, watch } from 'vue'
import { onBeforeRouteLeave, useRouter } from 'vue-router'

import type AccountsCard from '@/components/ui/AccountsCard.vue'
import EditSkinModal from '@/components/ui/skin/EditSkinModal.vue'
import VirtualSkinSectionList from '@/components/ui/skin/VirtualSkinSectionList.vue'
import { useNetworkStatus } from '@/composables/useNetworkStatus'
import { check_reachable, get_default_user, users } from '@/helpers/auth'
import type { RenderResult } from '@/helpers/rendering/batch-skin-renderer.ts'
import { skinBlobUrlMap } from '@/helpers/rendering/batch-skin-renderer.ts'
import { loadSkinArmorPreview, saveSkinArmorPreview } from '@/helpers/skin-armor-preview'
import type { Cape, Skin, SkinTextureUrl } from '@/helpers/skins.ts'
import {
	equip_skin,
	filterDefaultSkins,
	filterSavedSkins,
	flush_pending_skin_change,
	flush_pending_skin_change_for_profile,
	get_available_capes,
	get_available_skins,
	get_normalized_skin_texture,
	normalize_skin_texture,
	remove_custom_skin,
	save_custom_skin,
	set_custom_skin_order,
} from '@/helpers/skins.ts'
import { useTheming } from '@/store/state'

async function generateSkinPreviews(skins: Skin[], capes: Cape[]) {
	const { generateSkinPreviews: generate } =
		await import('@/helpers/rendering/skin-preview-renderer')
	await generate(skins, capes)
}

type VirtualSkinSectionListExpose = {
	getAddSkinButtonElement: () => HTMLElement | null | undefined
}

const PENDING_SKIN_REFRESH_DELAY_MS = 11_000
const DEFAULT_SKIN_SECTION_SORT_ORDER = ['Default skins']
const messages = defineMessages({
	skinSelectorTitle: {
		id: 'app.skins.title',
		defaultMessage: 'Skin selector',
	},
	createSkinButton: {
		id: 'app.skins.create-skin',
		defaultMessage: 'Create skin',
	},
	modrinthSection: {
		id: 'app.skins.section.modrinth',
		defaultMessage: 'Modrinth',
	},
	defaultSkinsSection: {
		id: 'app.skins.section.default-skins',
		defaultMessage: 'Default skins',
	},
	mineconEarth2017Section: {
		id: 'app.skins.section.minecon-earth-2017',
		defaultMessage: 'MINECON Earth 2017',
	},
	buildersAndBiomesSection: {
		id: 'app.skins.section.builders-and-biomes',
		defaultMessage: 'Builders & Biomes',
	},
	stridingHeroSection: {
		id: 'app.skins.section.striding-hero',
		defaultMessage: 'Striding Hero',
	},
	theGardenAwakensSection: {
		id: 'app.skins.section.the-garden-awakens',
		defaultMessage: 'The Garden Awakens',
	},
	chaseTheSkiesSection: {
		id: 'app.skins.section.chase-the-skies',
		defaultMessage: 'Chase the Skies',
	},
	theCopperAgeSection: {
		id: 'app.skins.section.the-copper-age',
		defaultMessage: 'The Copper Age',
	},
	mountsOfMayhemSection: {
		id: 'app.skins.section.mounts-of-mayhem',
		defaultMessage: 'Mounts of Mayhem',
	},
	tinyTakeoverSection: {
		id: 'app.skins.section.tiny-takeover',
		defaultMessage: 'Tiny Takeover',
	},
	chaosCubedSection: {
		id: 'app.skins.section.chaos-cubed',
		defaultMessage: 'Chaos Cubed',
	},
	rateLimitTitle: {
		id: 'app.skins.rate-limit.title',
		defaultMessage: 'Slow down!',
	},
	rateLimitText: {
		id: 'app.skins.rate-limit.text',
		defaultMessage:
			"You're changing your skin too frequently. Mojang's servers have temporarily blocked further requests. Please wait a moment before trying again.",
	},
	droppedFileErrorTitle: {
		id: 'app.skins.dropped-file-error.title',
		defaultMessage: 'Error processing file',
	},
	droppedFileErrorText: {
		id: 'app.skins.dropped-file-error.text',
		defaultMessage: 'Failed to read the dropped file.',
	},
	reorderSkinErrorTitle: {
		id: 'app.skins.reorder-error.title',
		defaultMessage: 'Failed to reorder skins',
	},
	reorderSkinErrorText: {
		id: 'app.skins.reorder-error.text',
		defaultMessage: 'Your skin order could not be saved.',
	},
	deleteSkinTitle: {
		id: 'app.skins.delete-modal.title',
		defaultMessage: 'Are you sure you want to delete this skin?',
	},
	deleteSkinDescription: {
		id: 'app.skins.delete-modal.description',
		defaultMessage: 'This will permanently delete the selected skin. This action cannot be undone.',
	},
	previewingBadge: {
		id: 'app.skins.previewing-badge',
		defaultMessage: 'Previewing',
	},
	applyButton: {
		id: 'app.skins.apply-button',
		defaultMessage: 'Apply',
	},
	editSkinButton: {
		id: 'app.skins.preview.edit-button',
		defaultMessage: 'Edit skin',
	},
	axolotlAlt: {
		id: 'app.skins.sign-in.axolotl-alt',
		defaultMessage: 'Axolotl Launcher',
	},
	signInTitle: {
		id: 'app.skins.sign-in.title',
		defaultMessage: 'Please sign in',
	},
	signInDescription: {
		id: 'app.skins.sign-in.description',
		defaultMessage:
			'Please sign into your Minecraft account to use the skin management features of Axolotl Launcher.',
	},
	signInButton: {
		id: 'app.skins.sign-in.button',
		defaultMessage: 'Sign In',
	},
	offlineCompatibility: {
		id: 'app.skins.offline-account.compatibility',
		defaultMessage:
			'Offline skins are applied locally with a resource pack. Compatibility is best on Minecraft 1.6–1.19.2; newer versions are supported on a best-effort basis. The skin is not uploaded to Mojang, may also replace other default-skinned players on your client, and some skin mods or server plugins may override it.',
	},
	offlineCompatibilityTitle: {
		id: 'app.skins.offline-account.compatibility-title',
		defaultMessage: 'Offline skin compatibility',
	},
	thirdPartyManagementTitle: {
		id: 'app.skins.third-party-account.title',
		defaultMessage: 'Third-party skin management',
	},
	thirdPartyManagementDescription: {
		id: 'app.skins.third-party-account.description',
		defaultMessage:
			'Skins for this account are managed by its Yggdrasil provider. Open the provider website to change skins or capes.',
	},
	savedTab: {
		id: 'app.skins.tabs.saved',
		defaultMessage: 'Saved skins',
	},
	defaultTab: {
		id: 'app.skins.tabs.default',
		defaultMessage: 'Official skins',
	},
})

const editSkinModal = useTemplateRef('editSkinModal')
const addSkinFileInput = useTemplateRef<HTMLInputElement>('addSkinFileInput')
const skinSectionList = useTemplateRef<VirtualSkinSectionListExpose>('skinSectionList')
const skinPreviewArea = useTemplateRef<HTMLElement>('skinPreviewArea')
const armorPreviewConfig = ref<ArmorPreviewConfig>(loadSkinArmorPreview())

watch(armorPreviewConfig, saveSkinArmorPreview, { deep: true })

const { formatMessage } = useVIntl()
const router = useRouter()
const notifications = injectNotificationManager()
const { addNotification, handleError } = notifications

const themeStore = useTheming()
const skins = ref<Skin[]>([])
const capes = ref<Cape[]>([])
const { browserOffline, offline, setNetworkReachable } = useNetworkStatus()

const accountsCard = inject('accountsCard') as Ref<typeof AccountsCard>
const accountChangeRevision = computed(() => accountsCard.value?.accountChangeRevision)
const currentUser = ref(undefined)
const currentUserId = ref<string | undefined>(undefined)
const currentAccountType = ref<'microsoft' | 'offline' | 'yggdrasil' | undefined>(undefined)

const username = computed(() => currentUser.value?.profile?.name ?? undefined)
const selectedSkin = ref<Skin | null>(null)
const isApplyingSkin = ref(false)

const originalSelectedSkin = ref<Skin | null>(null)

const savedSkins = computed(() => {
	try {
		return filterSavedSkins(skins.value)
	} catch (error) {
		handleError(error as Error)
		return []
	}
})
const authServerQuery = useQuery({
	queryKey: ['authServerReachability'],
	enabled: computed(() => !browserOffline.value),
	queryFn: async () => {
		try {
			await check_reachable()
			setNetworkReachable(true)
			return true
		} catch (error) {
			setNetworkReachable(false)
			throw error
		}
	},
	refetchInterval: 5 * 60 * 1000,
	retry: false,
	refetchOnWindowFocus: false,
})
const defaultSkins = computed(() => filterDefaultSkins(skins.value))
const defaultSkinSections = computed(() => {
	const sections = new Map<string, Skin[]>()

	for (const skin of defaultSkins.value) {
		const section = skin.section ?? 'Default skins'
		const sectionSkins = sections.get(section)

		if (sectionSkins) {
			sectionSkins.push(skin)
		} else {
			sections.set(section, [skin])
		}
	}

	return Array.from(sections, ([section, skins]) => ({
		section,
		title: getDefaultSkinSectionTitle(section),
		skins,
	})).sort(
		(a, b) => getDefaultSkinSectionSortIndex(a.section) - getDefaultSkinSectionSortIndex(b.section),
	)
})

const currentCape = computed(() => {
	if (selectedSkin.value?.cape_id) {
		const overrideCape = capes.value.find((c) => c.id === selectedSkin.value?.cape_id)
		if (overrideCape) {
			return overrideCape
		}
	}
	return undefined
})

const skinTexture = computedAsync(async () => {
	const skin = selectedSkin.value
	if (skin?.texture) {
		try {
			return await get_normalized_skin_texture(skin)
		} catch (error) {
			if (skin.texture.startsWith('data:image/')) {
				return skin.texture
			}

			handleError(error as Error)
			return ''
		}
	} else {
		return ''
	}
})
const capeTexture = computed(() => currentCape.value?.texture)
const skinVariant = computed(() => selectedSkin.value?.variant)
const skinNametag = computed(() => (themeStore.hideNametagSkinsPage ? undefined : username.value))
const isSkinManagementReadOnly = computed(
	() =>
		currentAccountType.value === 'yggdrasil' ||
		(currentAccountType.value !== 'offline' &&
			(offline.value || (authServerQuery.isError.value && !authServerQuery.isLoading.value))),
)
const hasPendingSkinChange = computed(
	() => !skinsMatch(selectedSkin.value, originalSelectedSkin.value),
)

let userCheckInterval: number | null = null
let pendingSkinRefreshTimeout: number | null = null
let isUnmounted = false
let unlistenNativeDrop: (() => void) | null = null
let skinNavigationRevision = 0
const skinNavigationRevisions = new WeakMap<object, number>()

const isDraggingSkinFile = ref(false)
const isAddSkinButtonDragActive = ref(false)

const deleteSkinModal = ref()
const skinToDelete = ref<Skin | null>(null)
const skinListTab = ref<'saved' | 'default'>('saved')

const skinListTabLinks = computed(() => [
	{
		href: 'saved',
		label: formatMessage(messages.savedTab),
	},
	{
		href: 'default',
		label: formatMessage(messages.defaultTab),
	},
])

function confirmDeleteSkin(skin: Skin) {
	if (isSkinManagementReadOnly.value) return

	skinToDelete.value = skin
	deleteSkinModal.value?.show()
}

async function deleteSkin() {
	if (isSkinManagementReadOnly.value) return

	const deletedSkin = skinToDelete.value
	if (!deletedSkin) return

	try {
		await remove_custom_skin(deletedSkin)
		removeLocalSkin(deletedSkin)
	} catch (error) {
		handleError(error as Error)
	} finally {
		skinToDelete.value = null
	}
}

async function loadCapes() {
	try {
		capes.value = (await get_available_capes()) ?? []
	} catch (error) {
		if (currentUser.value && error instanceof Error) {
			handleError(error)
		}
	}
}

async function loadSkins() {
	try {
		const loadedSkins = (await get_available_skins()) ?? []
		const loadedEquippedSkin = loadedSkins.find((s) => s.is_equipped)
		const locallyKnownEquippedSkin =
			originalSelectedSkin.value &&
			(loadedSkins.find((skin) => skinsMatch(skin, originalSelectedSkin.value)) ??
				(originalSelectedSkin.value.texture.startsWith('data:image/')
					? originalSelectedSkin.value
					: undefined))
		const shouldPreserveKnownEquippedSkin =
			isSkinManagementReadOnly.value &&
			locallyKnownEquippedSkin &&
			!skinsMatch(loadedEquippedSkin, locallyKnownEquippedSkin)

		skins.value =
			shouldPreserveKnownEquippedSkin && locallyKnownEquippedSkin
				? mergeEquippedSkin(loadedSkins, locallyKnownEquippedSkin)
				: loadedSkins
		generateSkinPreviews(skins.value, capes.value)
		selectedSkin.value = skins.value.find((s) => s.is_equipped) ?? null
		originalSelectedSkin.value = selectedSkin.value
	} catch (error) {
		if (currentUser.value && error instanceof Error) {
			handleError(error)
		}
	}
}

function mergeEquippedSkin(list: Skin[], equippedSkin: Skin) {
	let foundEquippedSkin = false
	const mergedSkins = list.map((skin) => {
		const isEquipped = skinsMatch(skin, equippedSkin)
		foundEquippedSkin ||= isEquipped

		return {
			...skin,
			is_equipped: isEquipped,
		}
	})

	if (!foundEquippedSkin) {
		mergedSkins.unshift({
			...equippedSkin,
			is_equipped: true,
		})
	}

	return mergedSkins
}

function skinsMatch(a?: Skin | null, b?: Skin | null) {
	return (
		a?.source === b?.source &&
		a?.texture_key === b?.texture_key &&
		a?.variant === b?.variant &&
		(a?.cape_id ?? null) === (b?.cape_id ?? null)
	)
}

function skinsMatchIgnoringSource(a?: Skin | null, b?: Skin | null) {
	return (
		a?.texture_key === b?.texture_key &&
		a?.variant === b?.variant &&
		(a?.cape_id ?? null) === (b?.cape_id ?? null)
	)
}

function isSkinSelected(skin: Skin) {
	return skinsMatch(selectedSkin.value, skin)
}

function isSkinActive(skin: Skin) {
	return hasPendingSkinChange.value && skinsMatch(originalSelectedSkin.value, skin)
}

function getErrorMessage(error: unknown) {
	return error instanceof Error ? error.message : String(error)
}

function isMinecraftSkinRateLimitError(error: unknown) {
	const message = getErrorMessage(error)
	return message.includes('429 Too Many Requests') || message.includes('client error (429')
}

function getDefaultSkinSectionTitle(section?: string) {
	switch (section) {
		case 'Modrinth':
			return formatMessage(messages.modrinthSection)
		case 'MINECON Earth 2017':
			return formatMessage(messages.mineconEarth2017Section)
		case 'Builders & Biomes':
			return formatMessage(messages.buildersAndBiomesSection)
		case 'Striding Hero':
			return formatMessage(messages.stridingHeroSection)
		case 'The Garden Awakens':
			return formatMessage(messages.theGardenAwakensSection)
		case 'Chase the Skies':
			return formatMessage(messages.chaseTheSkiesSection)
		case 'The Copper Age':
			return formatMessage(messages.theCopperAgeSection)
		case 'Mounts of Mayhem':
			return formatMessage(messages.mountsOfMayhemSection)
		case 'Tiny Takeover':
			return formatMessage(messages.tinyTakeoverSection)
		case 'Chaos Cubed':
			return formatMessage(messages.chaosCubedSection)
		case 'Default skins':
			return formatMessage(messages.defaultSkinsSection)
		default:
			return section ?? formatMessage(messages.defaultSkinsSection)
	}
}

function getDefaultSkinSectionSortIndex(section: string) {
	const index = DEFAULT_SKIN_SECTION_SORT_ORDER.indexOf(section)
	return index === -1 ? DEFAULT_SKIN_SECTION_SORT_ORDER.length : index
}

function changeSkin(newSkin: Skin) {
	if (isSkinManagementReadOnly.value) return

	selectedSkin.value = newSkin
}

function resetSelectedSkin() {
	selectedSkin.value =
		skins.value.find((skin) => skinsMatch(skin, originalSelectedSkin.value)) ??
		originalSelectedSkin.value
}

function removeLocalSkin(deletedSkin: Skin) {
	const nextSkins = skins.value.filter((skin) => !skinsMatch(skin, deletedSkin))
	skins.value = nextSkins

	if (selectedSkin.value && skinsMatch(selectedSkin.value, deletedSkin)) {
		selectedSkin.value =
			nextSkins.find((skin) => skinsMatch(skin, originalSelectedSkin.value)) ??
			nextSkins.find((skin) => skin.is_equipped) ??
			null
	}

	if (originalSelectedSkin.value && skinsMatch(originalSelectedSkin.value, deletedSkin)) {
		originalSelectedSkin.value = nextSkins.find((skin) => skin.is_equipped) ?? null
	}

	generateSkinPreviews(skins.value, capes.value)
}

function setLocallyEquippedSkin(skinToApply: Skin) {
	skins.value = skins.value.map((skin) => ({
		...skin,
		is_equipped: skinsMatch(skin, skinToApply),
	}))
	originalSelectedSkin.value =
		skins.value.find((skin) => skinsMatch(skin, skinToApply)) ?? skinToApply
	selectedSkin.value = originalSelectedSkin.value
	void accountsCard.value?.setEquippedSkin(originalSelectedSkin.value)
}

function insertLocalSkin(savedSkin: Skin) {
	const firstNonCustomSkinIndex = skins.value.findIndex((skin) => skin.source !== 'custom')

	if (firstNonCustomSkinIndex === -1) {
		skins.value = [...skins.value, savedSkin]
		return
	}

	const nextSkins = [...skins.value]
	nextSkins.splice(firstNonCustomSkinIndex, 0, savedSkin)
	skins.value = nextSkins
}

function updateLocalSkin(savedSkin: Skin, applied: boolean, previousSkin?: Skin) {
	let foundSkin = false
	const replacesSelectedSkin =
		selectedSkin.value?.texture_key === savedSkin.texture_key ||
		(previousSkin ? skinsMatch(selectedSkin.value, previousSkin) : false)
	const replacesOriginalSkin =
		originalSelectedSkin.value?.texture_key === savedSkin.texture_key ||
		(previousSkin ? skinsMatch(originalSelectedSkin.value, previousSkin) : false)

	skins.value = skins.value.map((skin) => {
		const isUpdatedSkin = skin.texture_key === savedSkin.texture_key
		const isPreviousSkin = previousSkin && skinsMatch(skin, previousSkin)

		if (isUpdatedSkin || isPreviousSkin) {
			foundSkin = true
			return {
				...savedSkin,
				is_equipped: applied || savedSkin.is_equipped,
			}
		}

		return {
			...skin,
			is_equipped: applied ? false : skin.is_equipped,
		}
	})

	if (!foundSkin) {
		insertLocalSkin({
			...savedSkin,
			is_equipped: applied || savedSkin.is_equipped,
		})
	}

	if (applied) {
		const locallyEquippedSkin =
			skins.value.find((skin) => skin.texture_key === savedSkin.texture_key) ?? savedSkin

		originalSelectedSkin.value = locallyEquippedSkin
		selectedSkin.value = locallyEquippedSkin
		void accountsCard.value?.setEquippedSkin(locallyEquippedSkin)
	} else {
		const locallySavedSkin =
			skins.value.find((skin) => skin.texture_key === savedSkin.texture_key) ?? savedSkin

		if (replacesSelectedSkin) {
			selectedSkin.value = locallySavedSkin
		}

		if (replacesOriginalSkin) {
			originalSelectedSkin.value = locallySavedSkin
		}
	}

	generateSkinPreviews(skins.value, capes.value)
}

async function reorderSavedSkins(orderedSkins: Skin[]) {
	const previousSkins = skins.value
	const previousSelectedSkin = selectedSkin.value
	const previousOriginalSelectedSkin = originalSelectedSkin.value
	const orderedTextureKeys = orderedSkins.map((skin) => skin.texture_key)
	const orderedTextureKeySet = new Set(orderedTextureKeys)
	const remainingSavedSkins = previousSkins.filter(
		(skin) => skin.source !== 'default' && !orderedTextureKeySet.has(skin.texture_key),
	)
	const defaultSkins = previousSkins.filter((skin) => skin.source === 'default')
	const nextSavedSkins = [...orderedSkins, ...remainingSavedSkins]

	skins.value = [...nextSavedSkins, ...defaultSkins]
	generateSkinPreviews(skins.value, capes.value)

	try {
		const persistedSavedSkins = await preserveExternalSkins(nextSavedSkins)

		if (persistedSavedSkins.some((skin, index) => skin !== nextSavedSkins[index])) {
			skins.value = [...persistedSavedSkins, ...defaultSkins]
			generateSkinPreviews(skins.value, capes.value)
		}

		await set_custom_skin_order(
			persistedSavedSkins
				.filter((skin) => skin.source === 'custom')
				.map((skin) => skin.texture_key),
		)
	} catch (error) {
		skins.value = previousSkins
		selectedSkin.value = previousSelectedSkin
		originalSelectedSkin.value = previousOriginalSelectedSkin
		generateSkinPreviews(skins.value, capes.value)
		addNotification({
			type: 'error',
			title: formatMessage(messages.reorderSkinErrorTitle),
			text: error instanceof Error ? error.message : formatMessage(messages.reorderSkinErrorText),
		})
		await loadSkins()
	}
}

async function preserveExternalSkins(skinsToPersist: Skin[]) {
	const preservedSkins: Skin[] = []

	for (const skin of skinsToPersist) {
		if (skin.source !== 'custom_external') {
			preservedSkins.push(skin)
			continue
		}

		const textureBlob = await normalize_skin_texture(skin.texture)
		const capeId = skin.cape_id ? capes.value.find((cape) => cape.id === skin.cape_id) : undefined
		const savedSkin = await save_custom_skin(skin, textureBlob, skin.variant, capeId, false)
		const preservedSkin: Skin = {
			...savedSkin,
			source: 'custom',
			is_equipped: skin.is_equipped,
		}

		if (skinsMatchIgnoringSource(selectedSkin.value, skin)) {
			selectedSkin.value = preservedSkin
		}

		if (skinsMatchIgnoringSource(originalSelectedSkin.value, skin)) {
			originalSelectedSkin.value = preservedSkin
			void accountsCard.value?.setEquippedSkin(preservedSkin)
		}

		preservedSkins.push(preservedSkin)
	}

	return preservedSkins
}

function schedulePendingSkinRefresh() {
	if (pendingSkinRefreshTimeout !== null) {
		window.clearTimeout(pendingSkinRefreshTimeout)
	}

	const pendingProfileId = currentUser.value?.profile?.id

	pendingSkinRefreshTimeout = window.setTimeout(async () => {
		pendingSkinRefreshTimeout = null

		if (isUnmounted) {
			return
		}

		try {
			if (pendingProfileId) {
				await flush_pending_skin_change_for_profile(pendingProfileId)
			} else {
				await flush_pending_skin_change()
			}
		} catch (error) {
			handleError(error as Error)
			schedulePendingSkinRefresh()
			return
		}

		if (accountsCard.value) {
			await accountsCard.value.refreshValues()
		}

		await loadCapes()
		await loadSkins()
	}, PENDING_SKIN_REFRESH_DELAY_MS)
}

async function applySelectedSkin() {
	const skinToApply = selectedSkin.value
	if (
		!skinToApply ||
		!hasPendingSkinChange.value ||
		isApplyingSkin.value ||
		isSkinManagementReadOnly.value
	)
		return

	isApplyingSkin.value = true
	try {
		await equip_skin(skinToApply)
		setLocallyEquippedSkin(skinToApply)
		schedulePendingSkinRefresh()
	} catch (error) {
		if (isMinecraftSkinRateLimitError(error)) {
			notifications.addNotification({
				type: 'error',
				title: formatMessage(messages.rateLimitTitle),
				text: formatMessage(messages.rateLimitText),
			})
		} else {
			handleError(error as Error)
		}
	} finally {
		isApplyingSkin.value = false
	}
}

async function onSkinSaved(options: { applied: boolean; skin?: Skin; previousSkin?: Skin }) {
	if (options.skin) {
		updateLocalSkin(options.skin, options.applied, options.previousSkin)
	}

	if (!options.skin) {
		await loadCapes()
		await loadSkins()
	}

	if (options.applied) {
		schedulePendingSkinRefresh()
	}
}

async function loadCurrentUser() {
	try {
		const defaultId = await get_default_user(offline.value)
		currentUserId.value = defaultId

		const allAccounts = await users(offline.value)
		const selectedAccount = allAccounts.find((acc) => acc.account_id === defaultId)
		currentAccountType.value = selectedAccount?.account_type
		currentUser.value = selectedAccount
	} catch (e) {
		handleError(e as Error)
		currentUser.value = undefined
		currentUserId.value = undefined
		currentAccountType.value = undefined
	}
}

async function refreshSelectedAccount() {
	await loadCurrentUser()
	await loadCapes()
	await loadSkins()
}

watch(accountChangeRevision, (revision, previousRevision) => {
	if (revision === undefined || previousRevision === undefined) return
	void refreshSelectedAccount()
})

function getBakedSkinTextures(skin: Skin): RenderResult | undefined {
	const key = `${skin.texture_key}+${skin.variant}+${skin.cape_id ?? 'no-cape'}`
	return skinBlobUrlMap.get(key)
}

async function login() {
	if (offline.value) return
	accountsCard.value?.login()
}

function openAddSkinFileBrowser() {
	if (isSkinManagementReadOnly.value) return

	addSkinFileInput.value?.click()
}

async function onAddSkinFileInputChange(e: Event) {
	if (isSkinManagementReadOnly.value) return

	const files = (e.target as HTMLInputElement).files
	const file = files?.[0]

	if (!file) {
		return
	}

	await processSkinFileBuffer(await file.arrayBuffer())

	if (addSkinFileInput.value) {
		addSkinFileInput.value.value = ''
	}
}

function isSkinImagePath(path: string) {
	return path.toLowerCase().endsWith('.png')
}

function isSkinFileDrag(event: DragEvent) {
	const items = Array.from(event.dataTransfer?.items ?? [])
	const files = Array.from(event.dataTransfer?.files ?? [])

	return (
		items.some((item) => item.kind === 'file' && item.type === 'image/png') ||
		files.some((file) => file.type === 'image/png' || isSkinImagePath(file.name))
	)
}

function onAddSkinDragOver(event: DragEvent) {
	if (isSkinManagementReadOnly.value) return

	if (!isSkinFileDrag(event)) {
		return
	}

	event.preventDefault()
	isAddSkinButtonDragActive.value = true
}

function onAddSkinDragLeave() {
	if (isSkinManagementReadOnly.value) return

	isAddSkinButtonDragActive.value = false
}

async function onAddSkinDrop(event: DragEvent) {
	if (isSkinManagementReadOnly.value) return

	isAddSkinButtonDragActive.value = false

	const file = Array.from(event.dataTransfer?.files ?? []).find(
		(file) => file.type === 'image/png' || isSkinImagePath(file.name),
	)

	if (!file) {
		return
	}

	await processSkinFileBuffer(await file.arrayBuffer())
}

async function processSkinFileBuffer(buffer: Uint8Array | ArrayBuffer) {
	if (isSkinManagementReadOnly.value) return

	const fakeEvent = new MouseEvent('click')
	const originalSkinTexUrl = `data:image/png;base64,` + arrayBufferToBase64(buffer)
	try {
		const skinTextureNormalized = await normalize_skin_texture(originalSkinTexUrl, true)
		const skinTexUrl: SkinTextureUrl = {
			original: originalSkinTexUrl,
			normalized: `data:image/png;base64,` + arrayBufferToBase64(skinTextureNormalized),
		}
		editSkinModal.value?.showNew(fakeEvent, skinTexUrl)
	} catch (error) {
		handleError(error as Error)
	}
}

/** Native drop handler: Tauri blocks HTML5 `drop` for OS-level file drags,
 *  so we need onDragDropEvent to receive the file. HTML5 dragenter/dragover
 *  still fire and provide visual feedback — this only handles the drop. */
async function setupNativeDropHandler() {
	try {
		unlistenNativeDrop = await getCurrentWebview().onDragDropEvent(
			(event: { payload: DragDropEvent }) => {
				const payload = event.payload
				if (payload.type !== 'drop') return
				if (isSkinManagementReadOnly.value) return

				const pngPath = (payload.paths as string[]).find((p: string) =>
					p.toLowerCase().endsWith('.png'),
				)
				if (!pngPath) return

				readDroppedSkinFile(pngPath)
			},
		)
	} catch (error) {
		console.warn('Failed to set up native drop handler on skin page', error)
	}
}

async function readDroppedSkinFile(path: string) {
	try {
		const data = await invoke<ArrayBuffer>('plugin:files|file_read_dragged_file', { path })
		await processSkinFileBuffer(new Uint8Array(data))
	} catch (error) {
		handleError(error as Error)
	}
}

watch(
	() => selectedSkin.value?.cape_id,
	() => {},
)

watch(isSkinManagementReadOnly, (readOnly) => {
	if (readOnly) {
		isDraggingSkinFile.value = false
		isAddSkinButtonDragActive.value = false
	}
})

const removeNavigationFailureHandler = router.afterEach((to, _from, failure) => {
	if (
		failure &&
		skinNavigationRevisions.get(to) === skinNavigationRevision &&
		skinPreviewArea.value
	) {
		skinPreviewArea.value.style.visibility = ''
	}
})

onBeforeRouteLeave(async (to) => {
	const previewArea = skinPreviewArea.value
	if (!previewArea) return true

	skinNavigationRevision += 1
	skinNavigationRevisions.set(to, skinNavigationRevision)
	previewArea.style.visibility = 'hidden'

	await new Promise<void>((resolve) => {
		window.requestAnimationFrame(() => window.requestAnimationFrame(() => resolve()))
	})
	return true
})

onMounted(() => {
	userCheckInterval = window.setInterval(checkUserChanges, 250)
	void setupNativeDropHandler()
})

onUnmounted(() => {
	isUnmounted = true
	removeNavigationFailureHandler()
	if (userCheckInterval !== null) {
		window.clearInterval(userCheckInterval)
	}

	if (unlistenNativeDrop) {
		unlistenNativeDrop()
		unlistenNativeDrop = null
	}

	if (pendingSkinRefreshTimeout !== null) {
		window.clearTimeout(pendingSkinRefreshTimeout)
		pendingSkinRefreshTimeout = null
	}
})

async function checkUserChanges() {
	try {
		const defaultId = await get_default_user(offline.value)
		if (defaultId !== currentUserId.value) {
			await refreshSelectedAccount()
		}
	} catch (error) {
		if (currentUser.value && error instanceof Error) {
			handleError(error)
		}
	}
}

await Promise.all([loadCapes(), loadCurrentUser()])
await loadSkins()
</script>

<template>
	<EditSkinModal
		ref="editSkinModal"
		:capes="capes"
		@saved="onSkinSaved"
		@deleted="() => loadSkins()"
	/>
	<input
		ref="addSkinFileInput"
		type="file"
		accept="image/png"
		class="hidden"
		@change="onAddSkinFileInputChange"
	/>
	<ConfirmModal
		ref="deleteSkinModal"
		:title="formatMessage(messages.deleteSkinTitle)"
		:description="formatMessage(messages.deleteSkinDescription)"
		:proceed-label="formatMessage(commonMessages.deleteLabel)"
		@proceed="deleteSkin"
	/>
	<Teleport
		v-if="currentUser && currentAccountType === 'offline'"
		to="#sidebar-default-teleport-target"
	>
		<section class="p-4">
			<h3 class="m-0 text-base font-semibold text-[var(--color-text-default)]">
				{{ formatMessage(messages.offlineCompatibilityTitle) }}
			</h3>
			<p class="mb-0 mt-2 text-sm leading-6 text-[var(--color-text-tertiary)]">
				{{ formatMessage(messages.offlineCompatibility) }}
			</p>
		</section>
	</Teleport>
	<Teleport
		v-if="currentUser && currentAccountType === 'yggdrasil'"
		to="#sidebar-default-teleport-target"
	>
		<section class="p-4">
			<h3 class="m-0 text-base font-semibold text-[var(--color-text-default)]">
				{{ formatMessage(messages.thirdPartyManagementTitle) }}
			</h3>
			<p class="mb-0 mt-2 text-sm leading-6 text-[var(--color-text-tertiary)]">
				{{ formatMessage(messages.thirdPartyManagementDescription) }}
			</p>
		</section>
	</Teleport>

	<div
		v-if="currentUser"
		data-onboarding-id="skins-page"
		class="skin-layout box-border min-h-full p-4"
	>
		<div class="sticky top-6 self-start p-2 pt-0">
			<h1 class="m-0 text-2xl font-bold flex items-center gap-2">
				{{ formatMessage(messages.skinSelectorTitle) }}
			</h1>
			<div
				ref="skinPreviewArea"
				class="ml-5 mt-4 flex h-[calc(60vh-1rem)] items-center justify-center max-[700px]:h-[calc(50vh-1rem)]"
			>
				<SkinPreviewRenderer
					v-model:armor-config="armorPreviewConfig"
					armor-preview
					:cape-src="capeTexture"
					:texture-src="skinTexture || ''"
					:variant="skinVariant"
					:nametag="skinNametag"
					:initial-rotation="Math.PI / 8"
				>
					<template v-if="hasPendingSkinChange" #nametag-badge>
						<div
							class="flex w-max shrink-0 items-center justify-center gap-1.5 whitespace-nowrap rounded-full bg-bg-blue px-3 py-1 text-sm font-semibold leading-5 text-brand-blue"
						>
							<EyeIcon class="size-5 shrink-0" />
							{{ formatMessage(messages.previewingBadge) }}
						</div>
					</template>
					<template #subtitle>
						<div
							v-if="hasPendingSkinChange"
							class="flex max-w-full flex-wrap items-center justify-center gap-2 px-2"
						>
							<button
								class="flex h-10 min-w-0 cursor-pointer items-center justify-center gap-2 rounded-[14px] border-0 bg-surface-4 px-4 py-2.5 text-base font-semibold leading-5 text-[var(--color-text-primary)] shadow-md transition-[filter,transform] duration-200 enabled:hover:brightness-[--hover-brightness] enabled:focus-visible:brightness-[--hover-brightness] enabled:active:scale-95 disabled:cursor-not-allowed disabled:opacity-50 [&>svg]:size-5 [&>svg]:shrink-0"
								:disabled="isApplyingSkin || isSkinManagementReadOnly"
								@click="resetSelectedSkin"
							>
								<RotateCounterClockwiseIcon />
								{{ formatMessage(commonMessages.resetButton) }}
							</button>
							<button
								class="flex h-10 min-w-0 cursor-pointer items-center justify-center gap-2 rounded-[14px] border-0 bg-brand px-4 py-2.5 text-base font-semibold leading-5 text-[rgba(0,0,0,0.9)] shadow-md transition-[filter,transform] duration-200 enabled:hover:brightness-[--hover-brightness] enabled:focus-visible:brightness-[--hover-brightness] enabled:active:scale-95 disabled:cursor-not-allowed disabled:opacity-50 [&>svg]:size-5 [&>svg]:shrink-0"
								:disabled="isApplyingSkin || isSkinManagementReadOnly"
								@click="applySelectedSkin"
							>
								<SpinnerIcon v-if="isApplyingSkin" class="animate-spin" />
								<CheckIcon v-else />
								{{ formatMessage(messages.applyButton) }}
							</button>
						</div>
						<button
							v-else
							class="flex h-10 min-w-0 cursor-pointer items-center justify-center gap-2 rounded-[14px] border-0 bg-surface-4 px-4 py-2.5 text-base font-semibold leading-5 shadow-md transition-[filter,transform] duration-200 enabled:hover:brightness-[--hover-brightness] enabled:focus-visible:brightness-[--hover-brightness] enabled:active:scale-95 disabled:cursor-not-allowed disabled:opacity-50 [&>svg]:size-5 [&>svg]:shrink-0"
							:disabled="!selectedSkin || isSkinManagementReadOnly"
							@click="(e: MouseEvent) => selectedSkin && editSkinModal?.show(e, selectedSkin)"
						>
							<EditIcon />
							{{ formatMessage(messages.editSkinButton) }}
						</button>
					</template>
				</SkinPreviewRenderer>
			</div>
		</div>

		<div class="pt-2">
			<div class="mb-4 flex flex-wrap items-center justify-between gap-3">
				<NavTabs
					:active-index="skinListTab === 'saved' ? 0 : 1"
					:links="skinListTabLinks"
					mode="local"
					@tab-click="
						(index: number) => {
							skinListTab = index === 0 ? 'saved' : 'default'
						}
					"
				/>
				<Button type="colored" color="brand" @click="router.push('/lab/skin-editor')"
					><PlusIcon />
					{{ formatMessage(messages.createSkinButton) }}
				</Button>
			</div>
			<VirtualSkinSectionList
				ref="skinSectionList"
				:active-tab="skinListTab"
				:saved-skins="savedSkins"
				:default-skin-sections="defaultSkinSections"
				:get-baked-skin-textures="getBakedSkinTextures"
				:is-skin-selected="isSkinSelected"
				:is-skin-active="isSkinActive"
				:is-add-skin-button-drag-active="isAddSkinButtonDragActive"
				:read-only="isSkinManagementReadOnly"
				@select="changeSkin"
				@edit="(skin, event) => editSkinModal?.show(event, skin)"
				@delete="confirmDeleteSkin"
				@reorder-saved-skins="reorderSavedSkins"
				@add-skin="openAddSkinFileBrowser"
				@add-skin-dragenter="onAddSkinDragOver"
				@add-skin-dragover="onAddSkinDragOver"
				@add-skin-dragleave="onAddSkinDragLeave"
				@add-skin-drop="onAddSkinDrop"
			/>
		</div>
	</div>

	<div
		v-else
		data-onboarding-id="skins-page"
		class="box-border flex min-h-full items-center justify-center pt-[25%]"
	>
		<div
			class="relative mx-auto flex w-full max-w-xl flex-col gap-5 rounded-lg bg-surface-3 p-7 shadow-lg"
		>
			<img
				src="@/assets/axolotl.png"
				:alt="formatMessage(messages.axolotlAlt)"
				class="absolute -top-32 right-8 h-36 w-36 object-contain md:right-20"
			/>
			<div
				class="absolute top-0 left-0 w-full h-[1px] opacity-40 bg-gradient-to-r from-transparent via-green-500 to-transparent"
				style="
					background: linear-gradient(
						to right,
						transparent 2rem,
						var(--color-green) calc(100% - 13rem),
						var(--color-green) calc(100% - 5rem),
						transparent calc(100% - 2rem)
					);
				"
			></div>

			<div class="flex flex-col gap-5">
				<h1 class="text-3xl font-extrabold m-0">
					{{ formatMessage(messages.signInTitle) }}
				</h1>
				<p class="text-lg m-0">
					{{ formatMessage(messages.signInDescription) }}
				</p>
				<Button
					v-if="!offline"
					v-show="accountsCard"
					type="colored"
					color="brand"
					:disabled="accountsCard.loginDisabled"
					@click="login"
					><LogInIcon v-if="!accountsCard.loginDisabled" />
					<SpinnerIcon v-else class="animate-spin" />
					{{ formatMessage(messages.signInButton) }}
				</Button>
			</div>
		</div>
	</div>
</template>

<style lang="scss" scoped>
.skin-layout {
	display: grid;
	grid-template-columns: minmax(0, 1fr) minmax(0, 2.5fr);
	gap: 2.5rem;

	@media (max-width: 700px) {
		grid-template-columns: 1fr;
	}
}
</style>
