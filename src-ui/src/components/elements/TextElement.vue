<template>
<BaseElement :instance="instance" :id="id" :template-id="templateId" :instance-template="instanceTemplate" :official="official" @state="updateState">
    <span class="content" v-html="content" />
</BaseElement>
</template>

<script setup lang="ts">
import BaseElement from './BaseElement.vue';
import { ElementInstance, ElementState, StateKeys, TextElementTemplate } from '../../types.ts';
import { setupPurifier } from '../../util.ts'
import { computed, onMounted, ref, watch } from 'vue';
import { useGlobalState } from '../../store/state.ts';
import Handlebars from 'handlebars'
import { DOMPurifyI } from 'dompurify';

const store = useGlobalState()

let purifier: DOMPurifyI

const emit = defineEmits<{
  (e: 'state', key: StateKeys, value: any): void
}>()

let content = ref<string>()
const props = defineProps<{
    instance: ElementInstance,
    id: string,
    instanceTemplate: TextElementTemplate,
    templateId: string,
    official?: boolean
}>()
const variables = computed(() => {
    return {
        ...store.variables,
        ...props.instance.variables
    }
})
let handlebarsTemplate: HandlebarsTemplateDelegate|null = null
function compileHandlebarsTemplate() {
    try {
        handlebarsTemplate = Handlebars.compile(props.instanceTemplate.hbTemplate)
        computeContent()
    } catch(err) {
        handlebarsTemplate = null
        content.value = "!!TEMPLATE FAILED!!"
        console.error( "template error:", ( err as any ).message )
        console.timeEnd( `compileTemplate:${props}` )
    }
} 
function computeContent() {
    if ( !handlebarsTemplate || !purifier ) return
    content.value = purifier.sanitize( handlebarsTemplate(variables.value) )
}

// watch(() => variables, computeContent)
watch(() => variables, compileHandlebarsTemplate)
watch(() => props.instanceTemplate.hbTemplate, compileHandlebarsTemplate)
onMounted( () => {
    purifier = setupPurifier(props.id)
    compileHandlebarsTemplate()
})

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