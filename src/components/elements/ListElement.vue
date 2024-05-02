<template>
<BaseElement :elem="elem" :state="state" :editable="editable" :interactable="interactable" @pos="updatePos" content-class="">
    <ul>
        <li v-for="(entry, i) in elem.list" :key="i" class="box mx-0 my-0">
            <h4 class="title is-4">{{ entry.title }}</h4>
            <span class="content" v-html="parseMarkdown(entry.content)"></span>
            <div class="buttons" v-if="entry.actions && interactable">
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
import { ElementState, ListElement } from '../../types.ts';
import { parseMarkdown } from '../../util.ts';

const emit = defineEmits<{
  (e: 'pos', x: number, y: number): void
}>()

const props = defineProps<{
    elem: ListElement,
    state?: ElementState,
    editable?: boolean,
    interactable?: boolean
}>()

function updatePos(x: number, y: number) {
    emit("pos", x, y)
}
</script>