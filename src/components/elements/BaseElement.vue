<template>
<div class="card root" ref="root" :style="style as StyleValue"@mouseleave="dropdownActive = false">
    <header ref="header" class="card-header" :style="{cursor: store.editable?'move':'inherit'}" v-if="elem.title" @mousedown="startDrag(false)" >
        <p :class="['card-header-title',textColorClass]">
            {{ elem.title }}
        </p>
        <div v-if="store.interactable" :class="['card-header-icon','dropdown','is-right',{'is-active': dropdownActive || colorPickerActive}]">
            <div class="dropdown-trigger"   @click.prevent="dropdownActive = !dropdownActive">
                <button class="" aria-haspopup="true" aria-controls="dropdown-menu">
                <Icon icon="fa-ellipsis" />
                </button>
            </div>
            <div class="dropdown-menu"role="menu">
                <div class="dropdown-content" @click="dropdownActive = false">
                    <a class="dropdown-item" @click="startDrag(true)">Move</a>
                    <a class="dropdown-item" @click="contentVisible = !contentVisible">{{ contentVisible ? 'Hide Content' : 'Show Content' }}</a>
                    <a class="dropdown-item" @click="colorPickerActive = !colorPickerActive">
                        Change Color
                        <span class="bd-color-swatch is-rounded" :style="contentStyle"></span>
                    </a>
                    <hr class="dropdown-divider" />
                    <a href="#" class="dropdown-item" @click="emit('state', '_reset', '*')">Reset</a>
                    <ColorPicker v-if="colorPickerActive" @choose="color => emit('state', 'bgColor', color)" />
                </div>
            </div>
        </div>
    </header>
    <div ref="body" :class="['card-body',contentClass??'card-content',textColorClass]" :style="contentStyle" v-if="contentVisible">
        <slot></slot>
        {{ getState('size', { width: 0, height: 0}) }}
    </div>
    <div class="resize-element-container">
        <div class="resize-element" v-if="store.editable" @mousedown="onResizeStart" @mouseup="onResizeStop">
            <Icon icon="fa-solid fa-up-right-and-down-left-from-center" rotate=90 />
        </div>
    </div>
</div>
</template>

<script setup lang="ts">
import { StyleValue, computed, onMounted, ref } from 'vue';
import { Color, ElementState, Position, StateKeys, UIElement } from '../../types.ts';
import { colorToCSS, shouldUseDarkText } from '../../util.ts';
import { useGlobalState } from '../../store/state.ts';
import ColorPicker from '../ColorPicker.vue'
const emit = defineEmits<{
  (e: 'state', key: StateKeys, value: any): void
}>()

const store = useGlobalState()


const props = defineProps<{
    elem: UIElement,
    state?: ElementState,

    contentClass?: string
}>()
let root = ref<HTMLElement>()
let body = ref<HTMLElement>()
let header = ref<HTMLElement>()
let contentVisible = ref(true)
let dragging = ref(false)
let dropdownActive = ref(false)
let colorPickerActive = ref(false)

const bgColor = computed(() => {
    return props.state?.bgColor ?? props.elem.defaults?.bgColor ?? { r : 255, g: 255, b: 255, a: 1 }
})
const textColorClass = computed(() => {
    return shouldUseDarkText(bgColor.value) ? "has-text-black" : "has-text-white"
})

function getState(key: keyof ElementState, defaultValue: any) {
    let value = props.state ? props.state[key] : undefined
    if(!value) value = props.elem.defaults ? props.elem.defaults[key] : undefined
    return value ?? defaultValue
}

const style = computed(() => {
    const size = getState("size", { width: 300, height: 400 })
    const pos = getState("position", { x: 40, y: 40})
    console.log({
        size: Object.assign({}, size), 
        body: { width: document.documentElement.clientWidth, height: document.documentElement.clientHeight }
    })
    // console.log(Math.min(size.width, document.documentElement.clientWidth), Math.min(size.height, document.documentElement.clientHeight))
    return {
        'background-color': colorToCSS(bgColor.value),
        display: dragging.value ? "hidden" : "block",
        position: "absolute",
        left: `${pos.x}px`,
        top: `${pos.y}px`,
        // width: Math.min(size.width, document.documentElement.clientWidth - pos.x) + "px",
        // width: Math.min(size.height, document.documentElement.clientHeight - pos.y) + "px", 
        // width: size.width + "px",
        // height: size.height + "px"
        // height: props.state?.size?.height ? `${props.state?.size?.height}px` : 'inherit',
    }
})

const contentStyle = computed(() => {
    const size = getState("size", { width: 200, height: 200 })
    return {
        'background-color': colorToCSS(bgColor.value),
        width: size.width + "px",
        height: size.height + "px"
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

    emit("state", "position", { x, y })
}
let dragStart: number | undefined
function startDrag(force = false) {
    if(dragging.value) return
    if(force || store.editable) {
        dropdownActive.value = false
        console.debug("startDrag")
        dragging.value = true
        dragStart = Date.now()
        window.addEventListener("mousemove", onMouseMove)
        window.addEventListener("mouseup", endDrag)
    }
} 
function endDrag() {
    if(!dragStart) return
    if(!dragging.value || Date.now() - dragStart < 5) return
    console.debug("endDrag")
    dragging.value = false
    window.removeEventListener("mouseup", endDrag)
    window.removeEventListener("mousemove", onMouseMove)
}

function onResizeStart(e: any) {
    document.addEventListener("mousemove", onResize)
    document.addEventListener("mouseup", onResizeStop)
}

function onResize(e: any) {
    const endPos = { x: e.x, y: e.y }
    const position = getState("position", { x: 0, y: 0})
    console.log(endPos, position)
    const distance = [endPos.x - position.x + 15, endPos.y - position.y + 15]
    console.log(`Math.max(Math.min(${distance[0]}, ${document.body.clientWidth - position.x}), ${300})`)
    console.log(`Math.max(Math.min(${distance[1]}, ${document.body.clientHeight - position.y}), ${150})`)
    const size = {
        width: Math.max(distance[0], 300),
        height: Math.max(distance[1], 150)
    }
    console.log("distance", size)
    emit("state", "size", size)
    e.preventDefault();
}
function onResizeStop(e: any) {
    document.removeEventListener("mousemove", onResize)
    document.removeEventListener("mouseup", onResizeStop)
}
</script>

<style scoped>
.root {
    min-width: 13em;
    min-height: 5rem;
}
.card-body {
    overflow-y: auto;
    overflow-x: clip;
    min-width: fit-content;
    min-height: fit-content;
    /* max-height: 10vh; */
}
.dropdown {
    overflow-y: visible;
}
.resize-element {
    position: relative;
    bottom: 0;
    right: 0;
    background-color: white;
    z-index: 1;
    cursor: nwse-resize;
    border-radius: 5px;
    user-select: none;
}

.resize-element-container {
    position: absolute;
    bottom: 0;
    right: 0;
}
</style>