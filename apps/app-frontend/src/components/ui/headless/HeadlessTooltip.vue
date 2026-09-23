<template>
	<TooltipProvider>
		<TooltipRoot :delay-duration="delayMs">
			<TooltipTrigger as-child>
				<slot />
			</TooltipTrigger>
			<TooltipPortal>
				<TooltipContent :side="side" :class="headlessTokenClasses.tooltipContent">
					<slot name="content" />
				</TooltipContent>
			</TooltipPortal>
		</TooltipRoot>
	</TooltipProvider>
</template>

<script setup lang="ts">
import {
	TooltipContent,
	TooltipPortal,
	TooltipProvider,
	TooltipRoot,
	TooltipTrigger,
} from 'reka-ui'

import { headlessTokenClasses } from './token-classes'

withDefaults(
	defineProps<{
		side?: 'top' | 'right' | 'bottom' | 'left'
		delayMs?: number
	}>(),
	{
		side: 'top',
		delayMs: 400,
	},
)
</script>
