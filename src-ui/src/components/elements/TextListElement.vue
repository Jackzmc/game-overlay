<template>
<BaseElement :elem="elem" :state="state" :official="official" @state="updateState" content-class="">
    <ul>
        <ListItem v-for="(item, i) in elem.list" :key="i" :item="item" />
    </ul>
</BaseElement>
</template>

<script setup lang="ts">
import BaseElement from './BaseElement.vue';
import { ElementState, TextListElement, StateKeys } from '../../types.ts';
import { useGlobalState } from '@/store/state.ts';
import ListItem from '../subelements/ListItem.vue'

const store = useGlobalState()

const emit = defineEmits<{
  (e: 'state', key: StateKeys, value: any): void
}>()

const props = defineProps<{
    elem: TextListElement,
    state?: ElementState,
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