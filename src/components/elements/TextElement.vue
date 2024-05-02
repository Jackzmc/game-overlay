<template>
<BaseElement :elem="elem" :state="state" :editable="editable" :interactable="interactable" @pos="updatePos">
    <span class="content" v-html="content" />
</BaseElement>
</template>

<script setup lang="ts">
import BaseElement from './BaseElement.vue';
import { ElementState, TextElement } from '../../types.ts';

import { parseMarkdown, replaceVariables } from '../../util.ts'
import { computed, inject, onMounted, ref, watch } from 'vue';
import { useGlobalState } from '../../store/state.ts';

const store = useGlobalState()

const emit = defineEmits<{
  (e: 'pos', x: number, y: number): void
}>()

let content = ref<string>()
const props = defineProps<{
    elem: TextElement,
    state?: ElementState,
    editable?: boolean,
    interactable?: boolean

}>()
async function computeContent() {
    const text = replaceVariables(props.elem.text, store.variables)
    content.value = await parseMarkdown(text)
}

function updatePos(x: number, y: number) {
    emit("pos", x, y)
}

watch(() => store.variables, computeContent)
watch(() => props.elem.text, computeContent)
onMounted(() => computeContent())
</script>

<style scoped>
.card-content {
    padding: 0;
}
</style>