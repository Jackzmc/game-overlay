<template>
<div>
    <div :class="['button','is-small',{'is-loading': state === State.Loading}]" :style="style" @click="performAction(false)">
        {{ action.label }}
    </div>
    <ConfirmModal v-if="state === State.Confirmation" @confirm="performAction(true)">
        <template #title>
            <b>Confirm Action: </b>{{action.label}}
        </template>
        <p>Are you sure you want to perform this action?</p>
        Action: <code>{{ action.action }}</code>
    </ConfirmModal>
</div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { Action, ActionFlags } from '../types';
import { colorToCSS } from '../util.ts';
import { invoke } from '@tauri-apps/api'

const props = defineProps<{
    action: Action
}>()

const style = computed(() => {
    return {
        'background-color': colorToCSS(props.action.bgColor),
    }
})

enum State { None, Confirmation, Loading }
let state = ref<State>(State.None)

async function performAction(force = false) {
    if(state.value === State.Loading) return
    console.debug(Object.assign({}, props.action))
    if(!force && props.action.flags && (props.action.flags & ActionFlags.RequireConfirmation)) {
        state.value = State.Confirmation
        return
    }
    state.value = State.Loading
    try {
        await invoke("perform_action", { action: props.action.action })
        state.value = State.None
    } catch(err) {
        // TODO: replace with modal or popup
        state.value = State.None
        alert("Action failed: " + (err as any).message)
    }
}
</script>