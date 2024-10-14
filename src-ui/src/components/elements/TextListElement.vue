<template>
<BaseElement :instance="instance"  :id="id" :template="template" :official="official" @state="updateState" content-class="">
    <ul>
        <ListItem v-for="(item, i) in template.list" :key="i" :item="item" />
    </ul>
</BaseElement>
</template>

<script setup lang="ts">
import BaseElement from './BaseElement.vue';
import { ElementState, TextListElement, StateKeys, ElementInstance, ElementTemplate, TextListElementTemplate } from '../../types.ts';
import { useGlobalState } from '@/store/state.ts';
import ListItem from '../subelements/ListItem.vue'

const store = useGlobalState()

const emit = defineEmits<{
  (e: 'state', key: StateKeys, value: any): void
}>()

const props = defineProps<{
    instance: ElementInstance,
    id: string,
    template: TextListElementTemplate,
    official?: boolean
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