<!-- 由 S4 集成 -->
<script setup lang="ts">
import {
	ChevronLeftIcon,
	ChevronRightIcon,
	SaveIcon,
	SpinnerIcon,
	WorldIcon,
} from '@modrinth/assets'
import { Avatar, Button, defineMessages, NewModal, useRelativeTime, useVIntl } from '@modrinth/ui'
import dayjs from 'dayjs'
import { ref, useTemplateRef } from 'vue'

import InstanceIcon from '@/components/ui/InstanceIcon.vue'
import { list } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types.d.ts'
import {
	get_instance_worlds,
	isSingleplayerWorld,
	type SingleplayerWorld,
	sortWorlds,
} from '@/helpers/worlds.ts'

export type RecipeWorldInstallTarget = {
	instanceId: string
	worldPath: string
}

const emit = defineEmits<{
	select: [target: RecipeWorldInstallTarget]
	saveAs: []
}>()

withDefaults(defineProps<{ showSaveAs?: boolean }>(), {
	showSaveAs: true,
})

const { formatMessage, locale } = useVIntl()
const formatRelativeTime = useRelativeTime()
const modal = useTemplateRef<InstanceType<typeof NewModal>>('modal')
const instances = ref<GameInstance[]>([])
const selectedInstance = ref<GameInstance | null>(null)
const worlds = ref<SingleplayerWorld[]>([])
const loading = ref(false)
const error = ref('')
const worldError = ref('')
const installingWorldPath = ref<string | null>(null)

const messages = defineMessages({
	title: {
		id: 'app.lab.recipe-generator.instance-export.title',
		defaultMessage: 'Install datapack into world',
	},
	chooseInstance: {
		id: 'app.lab.recipe-generator.instance-export.choose-instance',
		defaultMessage: 'Choose the instance that contains the world',
	},
	chooseWorld: {
		id: 'app.lab.recipe-generator.instance-export.choose-world',
		defaultMessage: 'Choose a singleplayer world',
	},
	back: {
		id: 'app.lab.recipe-generator.instance-export.back',
		defaultMessage: 'Back to instances',
	},
	noInstances: {
		id: 'app.lab.recipe-generator.instance-export.no-instances',
		defaultMessage: 'No installed instances are available.',
	},
	noWorlds: {
		id: 'app.lab.recipe-generator.instance-export.no-worlds',
		defaultMessage: 'This instance has no singleplayer worlds yet.',
	},
	installWorld: {
		id: 'app.lab.recipe-generator.instance-export.install-world',
		defaultMessage: 'Install datapack into {name}',
	},
	lastPlayed: {
		id: 'app.lab.recipe-generator.instance-export.last-played',
		defaultMessage: 'Played {ago}',
	},
	neverPlayed: {
		id: 'app.lab.recipe-generator.instance-export.never-played',
		defaultMessage: 'Not played yet',
	},
	saveAs: {
		id: 'app.lab.recipe-generator.instance-export.save-as',
		defaultMessage: 'Save as...',
	},
})

async function show(instanceId?: string) {
	selectedInstance.value = null
	worlds.value = []
	error.value = ''
	worldError.value = ''
	installingWorldPath.value = null
	loading.value = true
	modal.value?.show()
	try {
		const loaded = await list()
		instances.value = loaded
			.filter((instance) => instance.install_stage === 'installed')
			.sort((left, right) => {
				const lastPlayed =
					Number(new Date(right.last_played ?? 0)) - Number(new Date(left.last_played ?? 0))
				return lastPlayed || left.name.localeCompare(right.name, locale.value)
			})
		const initialInstance = instances.value.find((instance) => instance.id === instanceId)
		if (initialInstance) {
			await openInstance(initialInstance)
		}
	} catch (caught) {
		instances.value = []
		error.value = caught instanceof Error ? caught.message : String(caught)
	} finally {
		loading.value = false
	}
}

async function openInstance(instance: GameInstance) {
	selectedInstance.value = instance
	worlds.value = []
	worldError.value = ''
	installingWorldPath.value = null
	loading.value = true
	try {
		const loaded = await get_instance_worlds(instance.id)
		sortWorlds(loaded)
		worlds.value = loaded.filter(isSingleplayerWorld)
	} catch (caught) {
		worlds.value = []
		worldError.value = caught instanceof Error ? caught.message : String(caught)
	} finally {
		loading.value = false
	}
}

function backToInstances() {
	selectedInstance.value = null
	worlds.value = []
	worldError.value = ''
	installingWorldPath.value = null
}

async function installWorld(world: SingleplayerWorld) {
	const instance = selectedInstance.value
	if (!instance || installingWorldPath.value) return
	installingWorldPath.value = world.path
	emit('select', { instanceId: instance.id, worldPath: world.path })
	modal.value?.hide()
	installingWorldPath.value = null
}

function saveAs() {
	emit('saveAs')
	modal.value?.hide()
}

defineExpose({ show })
</script>

<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.title)"
		width="min(620px, calc(100vw - 2rem))"
		max-width="620px"
		scrollable
		max-content-height="min(38rem, 76vh)"
		actions-divider
	>
		<div class="flex min-h-[18rem] min-w-0 flex-col gap-4">
			<template v-if="!selectedInstance">
				<p class="m-0 text-sm text-secondary">{{ formatMessage(messages.chooseInstance) }}</p>
				<div v-if="loading" class="flex flex-1 items-center justify-center text-secondary">
					<SpinnerIcon class="size-6 animate-spin" />
				</div>
				<p
					v-else-if="error"
					class="m-0 flex flex-1 items-center justify-center text-center text-brand-red"
				>
					{{ error }}
				</p>
				<p
					v-else-if="instances.length === 0"
					class="m-0 flex flex-1 items-center justify-center text-center text-secondary"
				>
					{{ formatMessage(messages.noInstances) }}
				</p>
				<ul v-else class="m-0 flex list-none flex-col gap-1 p-0">
					<li v-for="instance in instances" :key="instance.id" class="min-w-0">
						<button
							type="button"
							class="flex min-h-16 w-full cursor-pointer items-center gap-3 rounded-lg border-0 bg-transparent px-3 py-2 text-left text-primary transition-colors hover:bg-button-bg focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-brand-shadow"
							@click="openInstance(instance)"
						>
							<InstanceIcon
								class="size-10 shrink-0"
								:icon-path="instance.icon_path"
								:instance-id="instance.id"
								:loader="instance.loader"
							/>
							<span class="flex min-w-0 flex-1 flex-col gap-0.5">
								<strong class="truncate text-contrast">{{ instance.name }}</strong>
								<span class="truncate text-sm capitalize text-secondary">
									{{ instance.game_version }} · {{ instance.loader }}
								</span>
							</span>
							<ChevronRightIcon class="size-5 shrink-0 text-secondary" aria-hidden="true" />
						</button>
					</li>
				</ul>
			</template>

			<template v-else>
				<div class="flex min-w-0 items-center gap-2">
					<Button type="quiet" size="2xs" @click="backToInstances"
						><ChevronLeftIcon />{{ formatMessage(messages.back) }}
					</Button>
					<strong class="min-w-0 truncate text-contrast">{{ selectedInstance.name }}</strong>
				</div>
				<p class="m-0 text-sm text-secondary">{{ formatMessage(messages.chooseWorld) }}</p>
				<div v-if="loading" class="flex flex-1 items-center justify-center text-secondary">
					<SpinnerIcon class="size-6 animate-spin" />
				</div>
				<p
					v-else-if="worldError"
					class="m-0 flex flex-1 items-center justify-center text-center text-brand-red"
				>
					{{ worldError }}
				</p>
				<p
					v-else-if="worlds.length === 0"
					class="m-0 flex flex-1 items-center justify-center text-center text-secondary"
				>
					{{ formatMessage(messages.noWorlds) }}
				</p>
				<ul v-else class="m-0 flex list-none flex-col gap-1 p-0">
					<li v-for="world in worlds" :key="world.path" class="min-w-0">
						<button
							type="button"
							class="flex min-h-16 w-full cursor-pointer items-center gap-3 rounded-lg border-0 bg-transparent px-3 py-2 text-left text-primary transition-colors hover:bg-button-bg focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-brand-shadow disabled:cursor-not-allowed disabled:opacity-60"
							:disabled="installingWorldPath !== null"
							:aria-label="formatMessage(messages.installWorld, { name: world.name })"
							@click="installWorld(world)"
						>
							<Avatar v-if="world.icon" class="size-10 shrink-0 rounded-lg" :src="world.icon" />
							<span
								v-else
								class="flex size-10 shrink-0 items-center justify-center rounded-lg bg-button-bg text-secondary"
							>
								<WorldIcon class="size-5" aria-hidden="true" />
							</span>
							<span class="flex min-w-0 flex-1 flex-col gap-0.5">
								<strong class="truncate text-contrast">{{ world.name }}</strong>
								<span class="truncate text-sm text-secondary">
									{{
										world.last_played
											? formatMessage(messages.lastPlayed, {
													ago: formatRelativeTime(dayjs(world.last_played).toISOString()),
												})
											: formatMessage(messages.neverPlayed)
									}}
								</span>
							</span>
							<SpinnerIcon
								v-if="installingWorldPath === world.path"
								class="size-5 shrink-0 animate-spin text-secondary"
							/>
							<ChevronRightIcon v-else class="size-5 shrink-0 text-secondary" aria-hidden="true" />
						</button>
					</li>
				</ul>
			</template>
		</div>

		<template #actions>
			<div v-if="showSaveAs" class="flex justify-end">
				<Button type="colored" color="brand" @click="saveAs"
					><SaveIcon />{{ formatMessage(messages.saveAs) }}
				</Button>
			</div>
		</template>
	</NewModal>
</template>
