<template>
<BaseElement :elem="elem" :state="state" @state="updateState" content-class="">
    <ul>
        <li v-for="(entry, i) in elem.list" :key="i" class="list-item mx-0 my-0">
            <h6 class="title is-6 mb-0">{{ entry.title }}</h6>
            <span class="content" v-html="parseMarkdown(entry.content)"></span>
            <div class="buttons mt-2" v-if="entry.actions && store.interactable">
                <ActionButton v-for="(action, i) in entry.actions" :key="i" :action="action">
                    {{ action.label }}
                </ActionButton>
            </div>
        </li>
    </ul>
</BaseElement>
</template>

<script setup lang="ts">
import BaseElement from './BaseElement.vue';
import ActionButton from '../ActionButton.vue';
import { ElementState, TextListElement, StateKeys } from '../../types.ts';
import { parseMarkdown } from '../../util.ts';
import { useGlobalState } from '../../store/state.ts';

const store = useGlobalState()

const emit = defineEmits<{
  (e: 'state', key: StateKeys, value: any): void
}>()

const props = defineProps<{
    elem: TextListElement,
    state?: ElementState,
}>()

function updateState(key: StateKeys, value: any) {
    emit("state", key, value)
}
</script>

<style scoped>
.list-item {
    padding: 5px;
    border-bottom: 0.1px solid lightgray;
}
</style>