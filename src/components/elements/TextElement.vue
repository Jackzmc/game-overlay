<template>
<BaseElement :elem="elem" :state="state" @state="updateState">
    <span class="content" v-html="content" />
</BaseElement>
</template>

<script setup lang="ts">
import BaseElement from './BaseElement.vue';
import { ElementState, StateKeys, TextElement } from '../../types.ts';

import { parseMarkdown, replaceVariables } from '../../util.ts'
import { computed, inject, onMounted, ref, watch } from 'vue';
import { useGlobalState } from '../../store/state.ts';

const store = useGlobalState()

const emit = defineEmits<{
  (e: 'state', key: StateKeys, value: any): void
}>()

let content = ref<string>()
const props = defineProps<{
    elem: TextElement,
    state?: ElementState,
}>()
async function computeContent() {
    const text = replaceVariables(props.elem.text, store.variables)
    content.value = await parseMarkdown(text)
}

watch(() => store.variables, computeContent)
watch(() => props.elem.text, computeContent)
onMounted(() => computeContent())

function updateState(key: StateKeys, value: any) {
    emit("state", key, value)
}
</script>

<style scoped>
.card-content {
    padding: 0;
}
</style>