<script setup lang="ts">
import { MessageIcon, SaveIcon, SparklesIcon, TextCursorInputIcon, XIcon } from '@modrinth/assets'
import {
	Button,
	Combobox,
	commonMessages,
	defineMessages,
	NewModal,
	Slider,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { computed, nextTick, ref } from 'vue'

import {
	HOME_GREETING_DEFAULT_FONT,
	HOME_GREETING_DEFAULT_FONT_SIZE,
	HOME_GREETING_DEFAULT_MODE,
	HOME_GREETING_FONT_SIZE_MAX,
	HOME_GREETING_FONT_SIZE_MIN,
	type HomeGreetingFont,
	type HomeGreetingMode,
	type HomeWidgetPlacement,
} from '@/components/home/home-dashboard'
import HomeGreeting from '@/components/home/HomeGreeting.vue'

const props = defineProps<{
	playerName: string | null
}>()

const emit = defineEmits<{
	save: [id: string, mode: HomeGreetingMode, text: string, font: HomeGreetingFont, fontSize: number]
}>()

const { formatMessage } = useVIntl()
const modal = ref<InstanceType<typeof NewModal>>()
const textInput = ref<InstanceType<typeof StyledInput>>()
const widgetId = ref('')
const mode = ref<HomeGreetingMode>(HOME_GREETING_DEFAULT_MODE)
const text = ref('')
const font = ref<HomeGreetingFont>(HOME_GREETING_DEFAULT_FONT)
const fontSize = ref(HOME_GREETING_DEFAULT_FONT_SIZE)

const messages = defineMessages({
	title: { id: 'app.home.greeting.settings.title', defaultMessage: 'Customize greeting' },
	modeLabel: { id: 'app.home.greeting.settings.mode', defaultMessage: 'Display style' },
	greetingMode: {
		id: 'app.home.greeting.settings.mode.greeting',
		defaultMessage: 'Greeting only',
	},
	greetingModeDescription: {
		id: 'app.home.greeting.settings.mode.greeting-description',
		defaultMessage: 'Show a rotating greeting based on the time of day.',
	},
	textAndGreetingMode: {
		id: 'app.home.greeting.settings.mode.text-and-greeting',
		defaultMessage: 'Text + greeting',
	},
	textAndGreetingModeDescription: {
		id: 'app.home.greeting.settings.mode.text-and-greeting-description',
		defaultMessage: 'Put your own message before the rotating greeting.',
	},
	textMode: {
		id: 'app.home.greeting.settings.mode.text',
		defaultMessage: 'Custom text only',
	},
	textModeDescription: {
		id: 'app.home.greeting.settings.mode.text-description',
		defaultMessage: 'Replace the automatic greeting with your own message.',
	},
	textLabel: { id: 'app.home.greeting.settings.text', defaultMessage: 'Custom text' },
	prefixPlaceholder: {
		id: 'app.home.greeting.settings.prefix-placeholder',
		defaultMessage: 'Welcome back, {name}.',
	},
	textPlaceholder: {
		id: 'app.home.greeting.settings.text-placeholder',
		defaultMessage: 'The next adventure starts here.',
	},
	textFallback: {
		id: 'app.home.greeting.settings.text-fallback',
		defaultMessage: 'Leave this empty to use the current automatic greeting.',
	},
	preview: { id: 'app.home.greeting.settings.preview', defaultMessage: 'Preview' },
	fontLabel: { id: 'app.home.greeting.settings.font', defaultMessage: 'Font' },
	fontSizeLabel: { id: 'app.home.greeting.settings.font-size', defaultMessage: 'Font size' },
	fontSans: { id: 'app.home.greeting.settings.font.sans', defaultMessage: 'Launcher' },
	fontMinecraft: { id: 'app.home.greeting.settings.font.minecraft', defaultMessage: 'Minecraft' },
	fontMono: { id: 'app.home.greeting.settings.font.mono', defaultMessage: 'Monospace' },
	fontSerif: { id: 'app.home.greeting.settings.font.serif', defaultMessage: 'Serif' },
})

const modeOptions = computed(() => [
	{
		id: 'greeting' as const,
		label: formatMessage(messages.greetingMode),
		description: formatMessage(messages.greetingModeDescription),
		icon: SparklesIcon,
	},
	{
		id: 'text-and-greeting' as const,
		label: formatMessage(messages.textAndGreetingMode),
		description: formatMessage(messages.textAndGreetingModeDescription),
		icon: MessageIcon,
	},
	{
		id: 'text' as const,
		label: formatMessage(messages.textMode),
		description: formatMessage(messages.textModeDescription),
		icon: TextCursorInputIcon,
	},
])

const placeholder = computed(() =>
	mode.value === 'text-and-greeting'
		? formatMessage(messages.prefixPlaceholder, { name: props.playerName ?? 'Steve' })
		: formatMessage(messages.textPlaceholder),
)

const fontOptions = computed(() => [
	{ value: 'sans' as const, label: formatMessage(messages.fontSans) },
	{ value: 'minecraft' as const, label: formatMessage(messages.fontMinecraft) },
	{ value: 'mono' as const, label: formatMessage(messages.fontMono) },
	{ value: 'serif' as const, label: formatMessage(messages.fontSerif) },
])

function selectMode(nextMode: HomeGreetingMode) {
	mode.value = nextMode
	if (nextMode !== 'greeting') void nextTick(() => textInput.value?.focus())
}

function show(widget: HomeWidgetPlacement) {
	widgetId.value = widget.id
	mode.value = widget.options?.greetingMode ?? HOME_GREETING_DEFAULT_MODE
	text.value = widget.options?.greetingText ?? ''
	font.value = widget.options?.greetingFont ?? HOME_GREETING_DEFAULT_FONT
	fontSize.value = widget.options?.greetingFontSize ?? HOME_GREETING_DEFAULT_FONT_SIZE
	modal.value?.show()
}

function save() {
	emit('save', widgetId.value, mode.value, text.value, font.value, fontSize.value)
	modal.value?.hide()
}

defineExpose({ show })
</script>

<template>
	<NewModal ref="modal" :header="formatMessage(messages.title)" width="560px" max-width="560px">
		<div class="flex min-w-0 flex-col gap-5">
			<section class="flex min-w-0 flex-col gap-2">
				<h3 class="m-0 text-sm font-semibold text-contrast">
					{{ formatMessage(messages.modeLabel) }}
				</h3>
				<div class="grid grid-cols-3 overflow-hidden rounded-lg border border-solid border-divider">
					<button
						v-for="option in modeOptions"
						:key="option.id"
						type="button"
						class="flex min-h-28 cursor-pointer flex-col items-start gap-2 border-0 border-r border-solid border-divider bg-transparent p-3 text-left last:border-r-0 hover:bg-button-bg focus-visible:z-[1] focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-brand-shadow"
						:class="{ 'bg-button-bg': mode === option.id }"
						:aria-pressed="mode === option.id"
						@click="selectMode(option.id)"
					>
						<component
							:is="option.icon"
							class="size-5"
							:class="mode === option.id ? 'text-brand' : 'text-secondary'"
							aria-hidden="true"
						/>
						<strong class="text-sm text-contrast">{{ option.label }}</strong>
						<span class="text-xs leading-4 text-secondary">{{ option.description }}</span>
					</button>
				</div>
			</section>

			<label v-if="mode !== 'greeting'" class="flex min-w-0 flex-col gap-2">
				<span class="text-sm font-semibold text-contrast">{{
					formatMessage(messages.textLabel)
				}}</span>
				<StyledInput
					ref="textInput"
					v-model="text"
					multiline
					:rows="2"
					:maxlength="120"
					:placeholder="placeholder"
					wrapper-class="w-full"
				/>
				<span class="text-xs text-secondary">{{ formatMessage(messages.textFallback) }}</span>
			</label>

			<section class="grid min-w-0 grid-cols-[minmax(0,0.8fr)_minmax(0,1.2fr)] gap-5">
				<label class="flex min-w-0 flex-col gap-2">
					<span class="text-sm font-semibold text-contrast">{{
						formatMessage(messages.fontLabel)
					}}</span>
					<Combobox v-model="font" :options="fontOptions" />
				</label>
				<label class="flex min-w-0 flex-col gap-2">
					<span class="text-sm font-semibold text-contrast">{{
						formatMessage(messages.fontSizeLabel)
					}}</span>
					<Slider
						v-model="fontSize"
						:min="HOME_GREETING_FONT_SIZE_MIN"
						:max="HOME_GREETING_FONT_SIZE_MAX"
						:step="1"
						unit="px"
					/>
				</label>
			</section>

			<section class="flex min-w-0 flex-col gap-2">
				<h3 class="m-0 text-sm font-semibold text-contrast">
					{{ formatMessage(messages.preview) }}
				</h3>
				<div class="min-h-28 rounded-lg bg-button-bg px-4 py-3">
					<HomeGreeting
						:player-name="playerName"
						:greeting-mode="mode"
						:greeting-text="text"
						:greeting-font="font"
						:greeting-font-size="fontSize"
						dashboard-size="2x1"
					/>
				</div>
			</section>
		</div>

		<template #actions>
			<div class="flex justify-end gap-2">
				<Button type="outlined" @click="modal?.hide()"
					><XIcon />
					{{ formatMessage(commonMessages.cancelButton) }}
				</Button>
				<Button type="colored" color="brand" @click="save"
					><SaveIcon />
					{{ formatMessage(commonMessages.saveChangesButton) }}
				</Button>
			</div>
		</template>
	</NewModal>
</template>
