<template>
<BaseElement :instance="instance" :id="id" :template="template" :official="official" @state="updateState">
    <span class="content" v-html="content" />
</BaseElement>
</template>

<script setup lang="ts">
import BaseElement from './BaseElement.vue';
import { ElementInstance, ElementState, StateKeys, TextElement, TextElementTemplate } from '../../types.ts';
import { useTemplate } from '../../util.ts'
import { computed, onMounted, ref, watch } from 'vue';
import { useGlobalState } from '../../store/state.ts';
import Handlebars from 'handlebars'

const store = useGlobalState()

const emit = defineEmits<{
  (e: 'state', key: StateKeys, value: any): void
}>()

let content = ref<string>()
const props = defineProps<{
    instance: ElementInstance,
    id: string,
    template: TextElementTemplate,
    official?: boolean
}>()
const variables = computed(() => {
    return {
        ...store.variables,
        ...props.instance.variables
    }
})
let handlebarsTemplate: HandlebarsTemplateDelegate|null = null
function compileTemplate() {
    try {
        console.time( `compileTemplate:${props.instance}` )
        handlebarsTemplate = Handlebars.compile(props.template.template)
        computeContent()
        console.timeEnd( `compileTemplate:${props}` )
    } catch(err) {
        handlebarsTemplate = null
        content.value = "!!TEMPLATE FAILED!!"
        console.error("template error:", (err as any).message)
    }
} 
function computeContent() {
    if(!handlebarsTemplate) return
    content.value = useTemplate(handlebarsTemplate, variables.value)
}

// watch(() => variables, computeContent)
watch(() => variables, compileTemplate)
watch(() => props.template.template, compileTemplate)
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