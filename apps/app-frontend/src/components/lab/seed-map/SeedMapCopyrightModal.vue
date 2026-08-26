<script setup lang="ts">
import { CodeIcon, ExternalIcon, ImageIcon, InfoIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, useVIntl } from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import { useTemplateRef } from 'vue'

import ModalWrapper from '@/components/ui/modal/ModalWrapper.vue'

const { formatMessage } = useVIntl()
const modal = useTemplateRef<InstanceType<typeof ModalWrapper>>('modal')

const messages = defineMessages({
	title: {
		id: 'app.lab.seed-map.copyright.title',
		defaultMessage: 'Copyright and attribution',
	},
	iconHeading: {
		id: 'app.lab.seed-map.copyright.icons-heading',
		defaultMessage: 'Map artwork',
	},
	iconBody: {
		id: 'app.lab.seed-map.copyright.icons-body',
		defaultMessage:
			'Some structure and biome icons used by this tool were sourced from MinecraftSearch. MinecraftSearch and the respective creators retain their rights in that artwork.',
	},
	visitMinecraftSearch: {
		id: 'app.lab.seed-map.copyright.visit-minecraft-search',
		defaultMessage: 'Visit MinecraftSearch',
	},
	engineHeading: {
		id: 'app.lab.seed-map.copyright.engine-heading',
		defaultMessage: 'Local map engine',
	},
	engineBody: {
		id: 'app.lab.seed-map.copyright.engine-body',
		defaultMessage:
			'Biome, terrain, spawn, and structure data is generated locally with cubiomes, Copyright (c) 2020 Cubitect, provided under the MIT License, together with the Axolotl native integration.',
	},
	viewCubiomes: {
		id: 'app.lab.seed-map.copyright.view-cubiomes',
		defaultMessage: 'View cubiomes',
	},
	disclaimerHeading: {
		id: 'app.lab.seed-map.copyright.disclaimer-heading',
		defaultMessage: 'Unofficial tool',
	},
	disclaimerBody: {
		id: 'app.lab.seed-map.copyright.disclaimer-body',
		defaultMessage:
			'Minecraft is a trademark of Mojang Synergies AB. Axolotl Launcher is not affiliated with or endorsed by Mojang or MinecraftSearch.',
	},
})

defineExpose({
	show: (event?: MouseEvent) => modal.value?.show(event),
})
</script>

<template>
	<ModalWrapper ref="modal" :header="formatMessage(messages.title)">
		<div class="copyright-notice">
			<section class="notice-section grid grid-cols-[1.5rem_minmax(0,1fr)] gap-3 border-b border-surface-5 pb-4">
				<ImageIcon aria-hidden="true" />
				<div>
					<h3>{{ formatMessage(messages.iconHeading) }}</h3>
					<p>{{ formatMessage(messages.iconBody) }}</p>
					<ButtonStyled size="small" type="outlined">
						<button @click="openUrl('https://minecraftsearch.com')">
							{{ formatMessage(messages.visitMinecraftSearch) }}
							<ExternalIcon />
						</button>
					</ButtonStyled>
				</div>
			</section>

			<section class="notice-section grid grid-cols-[1.5rem_minmax(0,1fr)] gap-3 border-b border-surface-5 pb-4">
				<CodeIcon aria-hidden="true" />
				<div>
					<h3>{{ formatMessage(messages.engineHeading) }}</h3>
					<p>{{ formatMessage(messages.engineBody) }}</p>
					<ButtonStyled size="small" type="outlined">
						<button @click="openUrl('https://github.com/Cubitect/cubiomes')">
							{{ formatMessage(messages.viewCubiomes) }}
							<ExternalIcon />
						</button>
					</ButtonStyled>
				</div>
			</section>

			<section class="notice-section grid grid-cols-[1.5rem_minmax(0,1fr)] gap-3 border-b border-surface-5 pb-4">
				<InfoIcon aria-hidden="true" />
				<div>
					<h3>{{ formatMessage(messages.disclaimerHeading) }}</h3>
					<p>{{ formatMessage(messages.disclaimerBody) }}</p>
				</div>
			</section>
		</div>
	</ModalWrapper>
</template>

<style scoped>
.copyright-notice {
	display: flex;
	width: min(34rem, calc(100vw - 3rem));
	flex-direction: column;
	gap: 1rem;
}

.notice-section:last-child {
	border-bottom: 0;
	padding-bottom: 0;
}

.notice-section > svg {
	width: 1.25rem;
	height: 1.25rem;
	margin-top: 0.1rem;
	color: var(--color-text-secondary);
}

.notice-section h3 {
	margin: 0;
	color: var(--color-text-primary);
	font-size: 0.95rem;
}

.notice-section p {
	margin: 0.35rem 0 0.75rem;
	color: var(--color-text-secondary);
	font-size: 0.8rem;
	line-height: 1.5;
}
</style>
