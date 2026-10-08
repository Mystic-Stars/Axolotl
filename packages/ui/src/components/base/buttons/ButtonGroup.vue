<template>
    <div data-button-group>
        <slot />
    </div>
</template>

<style scoped>
[data-button-group] {
    display: flex;
    gap: 1px;
}

[data-button-group] > :slotted(:not(:first-child)[data-button]),
[data-button-group] > :slotted(:not(:first-child)[data-button-group-item]),
[data-button-group] > :slotted(:not(:first-child)[data-button-group-item]) [data-button] {
    border-top-left-radius: 0;
    border-bottom-left-radius: 0;
}

[data-button-group] > :slotted(:not(:last-child)[data-button]),
[data-button-group] > :slotted(:not(:last-child)[data-button-group-item]),
[data-button-group] > :slotted(:not(:last-child)[data-button-group-item]) [data-button] {
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
}
</style>
