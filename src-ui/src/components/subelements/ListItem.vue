<template>
    <li class="list-item mx-0 my-0">
        <h6 class="title is-6 mb-0">{{ item.title }}</h6>
        <span class="content" v-html="content"></span>
        <div class="buttons mt-2" v-if="item.actions && store.interactable">
            <ActionButton v-for="(action, i) in item.actions" :key="i" :action="action">
                {{ action.label }}
            </ActionButton>
        </div>
    </li>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { TextListElementEntry } from '../../types.ts';
import { useGlobalState } from '@/store/state.ts';
import { useTemplate } from '../../util.ts';
import ActionButton from '../ActionButton.vue';
import Handlebars from 'handlebars'

const props = defineProps<{
    item: TextListElementEntry
}>()

const store = useGlobalState()

const template = computed(() => {
    return Handlebars.compile(props.item.content)
})

const content = computed(() => {
    if(!template.value) return ""
    return useTemplate(template.value, store.variables)
})
</script>