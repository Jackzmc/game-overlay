<template>
<BaseElement :elem="elem" :state="state" @pos="updatePos">
    <span class="content" v-html="content" />
</BaseElement>
</template>

<script setup lang="ts">
import BaseElement from './BaseElement.vue';
import { ElementState, TextElement } from '../../types.ts';

import { parseMarkdown } from '../../util.ts'
import { computed, onMounted, ref, watch } from 'vue';

const emit = defineEmits<{
  (e: 'pos', x: number, y: number): void
}>()

let content = ref<string>()
const props = defineProps<{
    elem: TextElement,
    state?: ElementState
}>()
async function computeContent() {
    content.value = await parseMarkdown(props.elem.text)
}

function updatePos(x: number, y: number) {
    emit("pos", x, y)
}

watch(() => props.elem.text, computeContent)
onMounted(() => computeContent())
</script>