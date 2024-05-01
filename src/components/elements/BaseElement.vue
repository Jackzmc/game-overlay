<template>
<div class="card root" ref="root" :style="style as StyleValue" @click="endDrag">
    <header ref="header" class="card-header" :style="{cursor: editable?'move':'inherit'}" v-if="elem.title" @click="startDrag(false)" @mouseleave="dropdownActive = false">
        <p class="card-header-title">
            {{ elem.title }}
        </p>
        <div class="card-header-icon dropdown is-active">
            <div class="dropdown-trigger">
                <button class="" aria-haspopup="true" aria-controls="dropdown-menu" @click.prevent="dropdownActive = !dropdownActive">
                <span class="icon is-small">
                    <i class="fas fa-angle-down" aria-hidden="true">...</i>
                </span>
                </button>
            </div>
            <div class="dropdown-menu" id="dropdown-menu" role="menu" v-if="dropdownActive">
                <div class="dropdown-content" @click="dropdownActive = false">
                    <a class="dropdown-item" @click="startDrag(true)">Move</a>
                    <a class="dropdown-item" @click="contentVisible = !contentVisible">{{ contentVisible ? 'Hide Content' : 'Show Content' }}</a>
                <!-- <hr class="dropdown-divider" />
                <a href="#" class="dropdown-item"> With a divider </a> -->
                </div>
            </div>
        </div>
    </header>
    <div class="card-content" v-if="contentVisible">
        <slot></slot>
    </div>
</div>
</template>

<script setup lang="ts">
import { StyleValue, computed, onMounted, ref } from 'vue';
import { ElementState, UIElement } from '../../types.ts';
const emit = defineEmits<{
  (e: 'pos', x: number, y: number): void
}>()


const props = defineProps<{
    elem: UIElement,
    state?: ElementState,
    editable?: boolean
}>()
let root = ref<HTMLElement>()
let header = ref<HTMLElement>()
let contentVisible = ref(true)
let dragging = ref(false)
let dropdownActive = ref(false)

const style = computed(() => {
    return {
        display: dragging.value ? "hidden" : "block",
        position: "absolute",
        left: `${props.state?.position?.x ?? props.elem.defaultPosition.x}px`,
        top: `${props.state?.position?.y ?? props.elem.defaultPosition.y}px`
    }
})

function onMouseMove(e: any) {
    if(!header.value) return
    const size = (header.value as HTMLElement).getBoundingClientRect()
    let x = e.clientX - (size.width/2)
    let y = e.clientY - (size.height/2)
    if(x < 0) x = 0
    else if(x >= document.body.clientWidth - size.width) x = document.body.clientWidth - size.width

    if(y < 0) y = 0
    else if(y >= document.documentElement.clientHeight - size.height) y = document.documentElement.clientHeight - size.height

    emit("pos", x, y)
}
let dragStart: number | undefined
function startDrag(force = false) {
    if(dragging.value) return
    if(force || props.editable) {
        console.debug("startDrag")
        dragging.value = true
        dragStart = Date.now()
        window.addEventListener("mousemove", onMouseMove)
    }
} 
function endDrag() {
    if(!dragStart) return
    console.log(Date.now(), dragStart, Date.now() - dragStart)
    if(!dragging.value || Date.now() - dragStart < 5) return
    console.debug("endDrag")
    dragging.value = false
    window.removeEventListener("mousemove", onMouseMove)
}
</script>

<style scoped>
.root {
    min-width: 10em;
}
</style>