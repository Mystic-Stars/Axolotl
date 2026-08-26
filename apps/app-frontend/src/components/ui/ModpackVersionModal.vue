<script setup>
import { CheckIcon } from '@modrinth/assets'
import { Badge, ButtonStyled, defineMessages, useVIntl } from '@modrinth/ui'
import { computed, ref } from 'vue'

import { SwapIcon } from '@/assets/icons/index.js'
import ModalWrapper from '@/components/ui/modal/ModalWrapper.vue'
import SymlinkInstanceWarning from '@/components/ui/SymlinkInstanceWarning.vue'
import { update_managed_modrinth_version } from '@/helpers/instance'
import { releaseColor } from '@/helpers/utils'

const props = defineProps({
	versions: {
		type: Array,
		required: true,
	},
	instance: {
		type: Object,
		default: null,
	},
})
const { formatMessage } = useVIntl()
const messages = defineMessages({
	changeVersion: {
		id: 'app.modpack.change-version',
		defaultMessage: 'Change modpack version',
	},
	name: { id: 'app.modpack.version-name', defaultMessage: 'Name' },
	supports: { id: 'app.modpack.version-supports', defaultMessage: 'Supports' },
})

defineExpose({
	show: () => {
		modpackVersionModal.value.show()
	},
})

const emit = defineEmits(['finish-install'])

const filteredVersions = computed(() => {
	return props.versions
})

const modpackVersionModal = ref(null)
const installedVersion = computed(() => props.instance?.link?.version_id)
const installing = computed(() => props.instance.install_stage !== 'installed')
const inProgress = ref(false)

const switchVersion = async (versionId) => {
	modpackVersionModal.value.hide()
	inProgress.value = true
	await update_managed_modrinth_version(props.instance.id, versionId)
	inProgress.value = false
	emit('finish-install')
}

const onHide = () => {
	if (!inProgress.value) {
		emit('finish-install')
	}
}
</script>

<template>
	<ModalWrapper
		ref="modpackVersionModal"
		class="modpack-version-modal"
		:header="formatMessage(messages.changeVersion)"
		:on-hide="onHide"
	>
		<div class="modal-body flex flex-col gap-3">
			<SymlinkInstanceWarning
				v-if="instance?.symlink_target"
				:symlink-target="instance.symlink_target"
			/>
			<div v-if="instance.link" class="mod-card">
				<div class="table border border-bg">
					<div class="table-row grid-cols-[min-content_1fr_1fr] table-head">
						<div class="table-cell table-text w-16 p-4" />
						<div class="name-cell table-cell table-text">
							{{ formatMessage(messages.name) }}
						</div>
						<div class="table-cell table-text">{{ formatMessage(messages.supports) }}</div>
					</div>
					<div class="overflow-y-auto max-h-[25rem]">
						<div
							v-for="version in filteredVersions"
							:key="version.id"
							class="table-row grid-cols-[min-content_1fr_1fr] selectable"
							@click="$router.push(`/project/${version.project_id}/version/${version.id}`)"
						>
							<div class="table-cell table-text">
								<ButtonStyled
									circular
									:color="version.id === installedVersion ? 'standard' : 'brand'"
								>
									<button
										:disabled="inProgress || installing || version.id === installedVersion"
										@click.stop="() => switchVersion(version.id)"
									>
										<SwapIcon v-if="version.id !== installedVersion" />
										<CheckIcon v-else />
									</button>
								</ButtonStyled>
							</div>
							<div class="name-cell table-cell table-text">
								<div class="version-link">
									{{ version.name.charAt(0).toUpperCase() + version.name.slice(1) }}
									<div class="version-badge">
										<div class="channel-indicator mr-2">
											<Badge
												:color="releaseColor(version.version_type)"
												:type="
													version.version_type.charAt(0).toUpperCase() +
													version.version_type.slice(1)
												"
											/>
										</div>
										<div>
											{{ version.version_number }}
										</div>
									</div>
								</div>
							</div>
							<div class="table-cell table-text stacked-text">
								<span>
									{{
										version.loaders
											.map((str) => str.charAt(0).toUpperCase() + str.slice(1))
											.join(', ')
									}}
								</span>
								<span>
									{{ version.game_versions.join(', ') }}
								</span>
							</div>
						</div>
					</div>
				</div>
			</div>
		</div>
	</ModalWrapper>
</template>

<style scoped lang="scss">
.card-row {
	display: flex;
	align-items: center;
	justify-content: space-between;
	background-color: var(--color-raised-bg);
}

.mod-card {
	display: flex;
	flex-direction: column;
	gap: 1rem;
	overflow: hidden;
	margin-top: 0.5rem;
}

.version-link {
	display: flex;
	flex-direction: column;
	gap: 0.25rem;

	.version-badge {
		display: flex;
		flex-wrap: wrap;
	}
}

.stacked-text {
	display: flex;
	flex-direction: column;
	gap: 0.25rem;
	align-items: flex-start;
}

</style>
