<template>
	<div
		ref="dropdown"
		tabindex="0"
		role="combobox"
		:aria-expanded="dropdownVisible"
		class="animated-dropdown"
		@focus="onFocus"
		@blur="onBlur"
		@focusout="onBlur"
		@mousedown.prevent
		@keydown.enter.prevent="toggleDropdown"
		@keydown.up.prevent="focusPreviousOption"
		@keydown.down.prevent="focusNextOptionOrOpen"
	>
		<div
			class="selected"
			:class="{
				disabled: disabled,
				'render-down': dropdownVisible && !effectiveRenderUp && !disabled,
				'render-up': dropdownVisible && effectiveRenderUp && !disabled,
			}"
			@click="toggleDropdown"
		>
			<div class="min-w-0 overflow-hidden">
				<slot :selected="selectedOption">
					<span>
						{{ selectedOption }}
					</span>
				</slot>
			</div>
			<DropdownIcon class="arrow" :class="{ rotate: dropdownVisible }" />
		</div>
		<div class="options-wrapper" :class="{ down: !effectiveRenderUp, up: effectiveRenderUp }">
			<transition name="options">
				<div
					v-show="dropdownVisible"
					class="options"
					role="listbox"
					:class="{ down: !effectiveRenderUp, up: effectiveRenderUp }"
					:style="automaticMenuStyle"
				>
					<div
						v-for="(option, index) in options"
						:key="index"
						ref="optionElements"
						tabindex="-1"
						role="option"
						:class="{ 'selected-option': selectedValue === option }"
						:aria-selected="selectedValue === option"
						class="option"
						@click="selectOption(option, index)"
						@keydown.space.prevent="selectOption(option, index)"
					>
						<input
							:id="`${name}-${index}`"
							v-model="radioValue"
							type="radio"
							:value="option"
							:name="name"
						/>
						<label :for="`${name}-${index}`">{{ getOptionLabel(option) }}</label>
					</div>
				</div>
			</transition>
		</div>
	</div>
</template>

<script setup>
import { DropdownIcon } from '@modrinth/assets'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import { dropdownPlacement } from './dropdown-placement'

const OPTION_HEIGHT_REM = 3
const DEFAULT_MAX_MENU_HEIGHT_REM = 18.75
const SAFE_GAP_REM = 0.5

const props = defineProps({
	options: {
		type: Array,
		required: true,
	},
	name: {
		type: String,
		required: true,
	},
	defaultValue: {
		type: [String, Number, Object],
		default: null,
	},
	placeholder: {
		type: [String, Number],
		default: null,
	},
	modelValue: {
		type: [String, Number, Object],
		default: null,
	},
	renderUp: {
		type: Boolean,
		default: false,
	},
	autoPlacement: {
		type: Boolean,
		default: false,
	},
	disabled: {
		type: Boolean,
		default: false,
	},
	displayName: {
		type: Function,
		default: undefined,
	},
	maxVisibleOptions: {
		type: Number,
		default: undefined,
	},
})

function getOptionLabel(option) {
	return props.displayName?.(option) ?? option
}

const emit = defineEmits(['input', 'change', 'update:modelValue'])

const dropdownVisible = ref(false)
const selectedValue = ref(props.modelValue || props.defaultValue)
const focusedOptionIndex = ref(null)
const dropdown = ref(null)
const optionElements = ref(null)
const automaticRenderUp = ref(false)
const automaticAvailableHeight = ref(null)
const effectiveRenderUp = computed(() =>
	props.autoPlacement ? automaticRenderUp.value : props.renderUp,
)
const automaticMenuStyle = computed(() =>
	props.autoPlacement && automaticAvailableHeight.value !== null
		? { maxHeight: `${automaticAvailableHeight.value}px` }
		: undefined,
)

function cssPixels(property, fallback = 0) {
	const value = Number.parseFloat(
		getComputedStyle(document.documentElement).getPropertyValue(property),
	)
	return Number.isFinite(value) ? value : fallback
}

function updatePlacement() {
	if (!props.autoPlacement || !dropdown.value) return
	const rect = dropdown.value.getBoundingClientRect()
	const rootFontSize = cssPixels('font-size', 16)
	const maxMenuHeight =
		(props.maxVisibleOptions ?? DEFAULT_MAX_MENU_HEIGHT_REM / OPTION_HEIGHT_REM) *
		OPTION_HEIGHT_REM *
		rootFontSize
	const expectedMenuHeight = Math.min(
		props.options.length * OPTION_HEIGHT_REM * rootFontSize,
		maxMenuHeight,
	)
	const placement = dropdownPlacement({
		viewportHeight: window.innerHeight,
		controlTop: rect.top,
		controlBottom: rect.bottom,
		floatingActionBarClearance: cssPixels('--floating-action-bar-clearance'),
		safeGap: SAFE_GAP_REM * rootFontSize,
		expectedMenuHeight,
	})
	automaticRenderUp.value = placement.renderUp
	automaticAvailableHeight.value = Math.min(expectedMenuHeight, placement.availableHeight)
}

function schedulePlacementUpdate() {
	if (!dropdownVisible.value) return
	void nextTick(updatePlacement)
}

const selectedOption = computed(() => {
	return getOptionLabel(selectedValue.value) ?? props.placeholder ?? 'Select an option'
})

const radioValue = computed({
	get() {
		return props.modelValue || selectedValue.value
	},
	set(newValue) {
		emit('update:modelValue', newValue)
		selectedValue.value = newValue
	},
})

watch(
	() => props.modelValue,
	(newValue) => {
		selectedValue.value = newValue
	},
)

const toggleDropdown = () => {
	if (!props.disabled) {
		if (!dropdownVisible.value) updatePlacement()
		dropdownVisible.value = !dropdownVisible.value
		dropdown.value.focus()
	}
}

const selectOption = (option, index) => {
	radioValue.value = option
	emit('change', { option, index })
	dropdownVisible.value = false
}

const onFocus = () => {
	if (!props.disabled) {
		updatePlacement()
		focusedOptionIndex.value = props.options.findIndex((option) => option === selectedValue.value)
		dropdownVisible.value = true
	}
}

onMounted(() => {
	if (!props.autoPlacement) return
	window.addEventListener('resize', schedulePlacementUpdate)
	window.addEventListener('scroll', schedulePlacementUpdate, true)
})

onBeforeUnmount(() => {
	if (!props.autoPlacement) return
	window.removeEventListener('resize', schedulePlacementUpdate)
	window.removeEventListener('scroll', schedulePlacementUpdate, true)
})

watch(() => props.options.length, schedulePlacementUpdate)

const onBlur = (event) => {
	if (!isChildOfDropdown(event.relatedTarget)) {
		dropdownVisible.value = false
	}
}

const focusPreviousOption = () => {
	if (!props.disabled) {
		if (!dropdownVisible.value) {
			toggleDropdown()
		}
		focusedOptionIndex.value =
			(focusedOptionIndex.value + props.options.length - 1) % props.options.length
		optionElements.value[focusedOptionIndex.value].focus()
	}
}

const focusNextOptionOrOpen = () => {
	if (!props.disabled) {
		if (!dropdownVisible.value) {
			toggleDropdown()
		}
		focusedOptionIndex.value = (focusedOptionIndex.value + 1) % props.options.length
		optionElements.value[focusedOptionIndex.value].focus()
	}
}

const isChildOfDropdown = (element) => {
	let currentNode = element
	while (currentNode) {
		if (currentNode === dropdown.value) {
			return true
		}
		currentNode = currentNode.parentNode
	}
	return false
}
</script>

<style lang="scss" scoped>
.animated-dropdown {
	width: 20rem;
	max-width: 100%;
	height: 40px;
	position: relative;
	display: inline-block;

	&:focus {
		outline: 0;
	}

	.selected {
		height: 100%;
		width: 100%;

		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--gap-sm) var(--gap-lg);
		background-color: var(--color-button-bg);
		gap: var(--gap-md);
		cursor: pointer;
		user-select: none;
		border-radius: var(--radius-md);
		box-shadow:
			var(--shadow-inset-sm),
			0 0 0 0 transparent;

		transition: 0.05s;

		span {
			display: block;
			overflow: hidden;
			text-overflow: ellipsis;
			white-space: nowrap;
		}

		&:not(.render-down):not(.render-up) {
			transition-delay: 0.2s;
		}

		&.disabled {
			cursor: not-allowed;
			filter: grayscale(50%);
			opacity: 0.5;
		}

		&.render-up {
			border-radius: 0 0 var(--radius-md) var(--radius-md);
		}

		&.render-down {
			border-radius: var(--radius-md) var(--radius-md) 0 0;
		}

		&:focus {
			outline: 0;
			filter: brightness(1.25);
			transition: filter 0.1s ease-in-out;
		}

		.arrow {
			transition: transform 0.2s ease;

			&.rotate {
				transform: rotate(180deg);
			}
		}
	}

	.options {
		z-index: 10;
		max-height: v-bind('maxVisibleOptions ? `calc(${maxVisibleOptions} * 3rem)` : "18.75rem"');
		overflow-y: auto;
		box-shadow:
			var(--shadow-inset-sm),
			0 0 0 0 transparent;

		.option {
			background-color: var(--color-button-bg);
			display: flex;
			align-items: center;
			padding: var(--gap-md);
			cursor: pointer;
			user-select: none;

			> label {
				cursor: pointer;
				min-width: 0;
				overflow-wrap: anywhere;
			}

			&:hover {
				filter: brightness(0.85);
				transition: filter 0.2s ease-in-out;
			}

			&:focus {
				outline: 0;
				filter: brightness(0.85);
				transition: filter 0.2s ease-in-out;
			}

			&.selected-option {
				background-color: var(--color-brand);
				color: var(--color-accent-contrast);
				font-weight: bolder;
			}

			input {
				display: none;
			}
		}
	}
}

.options-enter-active,
.options-leave-active {
	transition: transform 0.2s ease;
}

.options-enter-from,
.options-leave-to {
	// this is not 100% due to a safari bug
	&.up {
		transform: translateY(99.999%);
	}

	&.down {
		transform: translateY(-99.999%);
	}
}

.options-enter-to,
.options-leave-from {
	&.up {
		transform: translateY(0%);
	}
}

.options-wrapper {
	position: absolute;
	width: 100%;
	overflow-x: hidden;
	z-index: 9;

	&.up {
		top: 0;
		transform: translateY(-99.999%);
		border-radius: var(--radius-md) var(--radius-md) 0 0;
	}

	&.down {
		border-radius: 0 0 var(--radius-md) var(--radius-md);
	}
}
</style>
