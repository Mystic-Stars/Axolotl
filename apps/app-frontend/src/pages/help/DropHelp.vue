<template>
	<div class="flex flex-col gap-3 p-6">
		<h1 class="text-lg font-bold text-[var(--color-text-primary)]">
			{{ formatMessage(messages.title) }}
		</h1>

		<div class="flex flex-col gap-2">
			<BigOptionButton
				v-for="section in sections"
				:key="section.key"
				:icon="section.icon"
				:title="formatMessage(section.titleMsg)"
				:description="formatMessage(section.descMsg)"
				:note="formatMessage(section.formatsMsg)"
				no-icon-border
			/>
		</div>
	</div>
</template>

<script setup lang="ts">
import {
	BookIcon,
	BracesIcon,
	FolderOpenIcon,
	GridIcon,
	MapIcon,
	PackageOpenIcon,
	PaletteIcon,
	SparklesIcon,
} from '@modrinth/assets'
import { BigOptionButton, defineMessages, useVIntl } from '@modrinth/ui'
import type { Component } from 'vue'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: {
		id: 'drop.help.title',
		defaultMessage: 'What can I drop into Axolotl?',
	},
	instancesTitle: {
		id: 'drop.help.instances-title',
		defaultMessage: 'Launcher Instances',
	},
	instancesDesc: {
		id: 'drop.help.instances-desc',
		defaultMessage:
			'Drag folders from other launchers (MultiMC, Prism Launcher, HMCL, etc.) to import Minecraft instances.',
	},
	instancesFormats: {
		id: 'drop.help.instances-formats',
		defaultMessage: 'MultiMC / Prism Launcher / PolyMC / HMCL instance folders',
	},
	modsTitle: {
		id: 'drop.help.mods-title',
		defaultMessage: 'Mods',
	},
	modsDesc: {
		id: 'drop.help.mods-desc',
		defaultMessage: 'Drag mod JAR files to install them into a Minecraft instance.',
	},
	modsFormats: {
		id: 'drop.help.mods-formats',
		defaultMessage: '.jar',
	},
	modpacksTitle: {
		id: 'drop.help.modpacks-title',
		defaultMessage: 'Modpacks',
	},
	modpacksDesc: {
		id: 'drop.help.modpacks-desc',
		defaultMessage: 'Drag modpack ZIP/MRPACK files to create a new instance.',
	},
	modpacksFormats: {
		id: 'drop.help.modpacks-formats',
		defaultMessage: '.mrpack / .zip',
	},
	resourcePacksTitle: {
		id: 'drop.help.resource-packs-title',
		defaultMessage: 'Resource Packs',
	},
	resourcePacksDesc: {
		id: 'drop.help.resource-packs-desc',
		defaultMessage: 'Drag resource pack ZIP files to install them into an instance.',
	},
	resourcePacksFormats: {
		id: 'drop.help.resource-packs-formats',
		defaultMessage: '.zip',
	},
	dataPacksTitle: {
		id: 'drop.help.data-packs-title',
		defaultMessage: 'Data Packs',
	},
	dataPacksDesc: {
		id: 'drop.help.data-packs-desc',
		defaultMessage: "Drag data pack ZIP files to install them into a world's datapacks folder.",
	},
	dataPacksFormats: {
		id: 'drop.help.data-packs-formats',
		defaultMessage: '.zip',
	},
	shaderPacksTitle: {
		id: 'drop.help.shader-packs-title',
		defaultMessage: 'Shader Packs',
	},
	shaderPacksDesc: {
		id: 'drop.help.shader-packs-desc',
		defaultMessage: 'Drag shader pack ZIP files to install them into an instance.',
	},
	shaderPacksFormats: {
		id: 'drop.help.shader-packs-formats',
		defaultMessage: '.zip',
	},
	worldsTitle: {
		id: 'drop.help.worlds-title',
		defaultMessage: 'World Saves',
	},
	worldsDesc: {
		id: 'drop.help.worlds-desc',
		defaultMessage: 'Drag world save folders to import them into an instance.',
	},
	worldsFormats: {
		id: 'drop.help.worlds-formats',
		defaultMessage: 'Minecraft world save folder (level.dat)',
	},
	schematicsTitle: {
		id: 'drop.help.schematics-title',
		defaultMessage: 'Schematics',
	},
	schematicsDesc: {
		id: 'drop.help.schematics-desc',
		defaultMessage: 'Drag .litematic files to import them as schematics.',
	},
	schematicsFormats: {
		id: 'drop.help.schematics-formats',
		defaultMessage: '.litematic',
	},
})

interface HelpSection {
	key: string
	icon: Component
	titleMsg: { id: string; defaultMessage: string }
	descMsg: { id: string; defaultMessage: string }
	formatsMsg: { id: string; defaultMessage: string }
}

const sections: HelpSection[] = [
	{
		key: 'instances',
		icon: FolderOpenIcon,
		titleMsg: messages.instancesTitle,
		descMsg: messages.instancesDesc,
		formatsMsg: messages.instancesFormats,
	},
	{
		key: 'mods',
		icon: PackageOpenIcon,
		titleMsg: messages.modsTitle,
		descMsg: messages.modsDesc,
		formatsMsg: messages.modsFormats,
	},
	{
		key: 'modpacks',
		icon: GridIcon,
		titleMsg: messages.modpacksTitle,
		descMsg: messages.modpacksDesc,
		formatsMsg: messages.modpacksFormats,
	},
	{
		key: 'resource-packs',
		icon: PaletteIcon,
		titleMsg: messages.resourcePacksTitle,
		descMsg: messages.resourcePacksDesc,
		formatsMsg: messages.resourcePacksFormats,
	},
	{
		key: 'data-packs',
		icon: BracesIcon,
		titleMsg: messages.dataPacksTitle,
		descMsg: messages.dataPacksDesc,
		formatsMsg: messages.dataPacksFormats,
	},
	{
		key: 'shader-packs',
		icon: SparklesIcon,
		titleMsg: messages.shaderPacksTitle,
		descMsg: messages.shaderPacksDesc,
		formatsMsg: messages.shaderPacksFormats,
	},
	{
		key: 'worlds',
		icon: MapIcon,
		titleMsg: messages.worldsTitle,
		descMsg: messages.worldsDesc,
		formatsMsg: messages.worldsFormats,
	},
	{
		key: 'schematics',
		icon: BookIcon,
		titleMsg: messages.schematicsTitle,
		descMsg: messages.schematicsDesc,
		formatsMsg: messages.schematicsFormats,
	},
]
</script>
