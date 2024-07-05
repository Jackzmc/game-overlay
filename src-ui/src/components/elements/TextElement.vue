<template>
<BaseElement :elem="elem" :state="state" :official="official" @state="updateState">
    <span class="content" v-html="content" />
</BaseElement>
</template>

<script setup lang="ts">
import BaseElement from './BaseElement.vue';
import { ElementState, StateKeys, TextElement } from '../../types.ts';
import { useTemplate } from '../../util.ts'
import { computed, onMounted, ref, watch } from 'vue';
import { useGlobalState } from '@/store/state.ts';
import Handlebars from 'handlebars'

const store = useGlobalState()

const emit = defineEmits<{
  (e: 'state', key: StateKeys, value: any): void
}>()

let content = ref<string>()
const props = defineProps<{
    elem: TextElement,
    state?: ElementState,
    official?: boolean
}>()
const variables = computed(() => {
    return {
        ...store.variables,
        ...props.elem.variables
    }
})
let template: HandlebarsTemplateDelegate|null = null
function compileTemplate() {
    try {
        template = Handlebars.compile(props.elem.template, { })
        computeContent()
    } catch(err) {
        template = null
        content.value = "!!TEMPLATE FAILED!!"
        console.error("template error:", (err as any).message)
    }
} 
function computeContent() {
    if(!template) return
    content.value = useTemplate(template, variables.value)
}

watch(() => variables, computeContent)
watch(() => props.elem.template, compileTemplate)
onMounted(() => compileTemplate())

function updateState(key: StateKeys, value: any) {
    emit("state", key, value)
}
</script>

<style scoped>
.card-content {
    padding: 0;
}
.list-item {
    padding: 5px;
    border-bottom: 0.1px solid lightgray;
    margin-left: 0;
    margin-right: 0;
}
</style>