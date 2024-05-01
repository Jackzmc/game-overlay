<template>
<BaseElement :elem="elem" :state="state" @pos="updatePos">
    <ul>
        <li v-for="(entry, i) in elem.list" :key="i">
            <h4 class="title is-4">{{ entry.title }}</h4>
            {{ entry.content }}
        </li>
    </ul>
</BaseElement>
</template>

<script setup lang="ts">
import BaseElement from './BaseElement.vue';
import { ElementState, ListElement } from '../../types.ts';

const emit = defineEmits<{
  (e: 'pos', x: number, y: number): void
}>()

const props = defineProps<{
    elem: ListElement,
    state?: ElementState
}>()

function updatePos(x: number, y: number) {
    emit("pos", x, y)
}
</script>