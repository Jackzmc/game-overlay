<template>
<div :class="['card','root',stateClass,{'official': official, 'invisible': store.interactable&&visibility==ElemVisibility.InteractableOnly}]" ref="root" :style="style as StyleValue"@mouseleave="dropdownActive = false" v-if="isVisible">
    <header ref="header" class="card-header" :style="{cursor: store.editable?'move':'inherit'}" v-if="store.interactable">
        <p :class="['card-header-title',textColorClass]" @mousedown="startDrag(false)" >
            {{ title }}
            <span v-if="store.editable" class="tag ml-1" style="font-size:12px"> {{ size.width }}x{{ size.height }}</span>
        </p>
        <button class="" aria-haspopup="true" @click="toggleVisibility">
            <Icon :icon="visibility == ElemVisibility.Always ? 'fa-eye' : 'fa-eye-slash'" />
        </button>
        <div :class="['card-header-icon','dropdown','is-right',stateClass,{'is-active': dropdownActive || colorPickerActive}]">
            <div class="dropdown-trigger"   @click.prevent="dropdownActive = !dropdownActive">
                <button class="" aria-haspopup="true" aria-controls="dropdown-menu">
                    <Icon icon="fa-ellipsis" />
                </button>
            </div>
            <div class="dropdown-menu"role="menu">
                <div class="dropdown-content" @click="dropdownActive = false">
                    <a class="dropdown-item" @click="startDrag(true)">Move</a>
                    <a class="dropdown-item" @click="toggleVisibility">{{ visibility == ElemVisibility.Always ? 'Hide' : 'Show' }}</a>
                    <a class="dropdown-item" @click="colorPickerActive = !colorPickerActive">
                        Change Color
                        <span class="bd-color-swatch is-rounded" :style="contentStyle"></span>
                    </a>
                    <div class="dropdown-item">
                        <label class="label">Opacity</label>
                        <input class="slider is-fullwidth" step="1" min="0" max="100" :value="bgColor.a! * 100" type="range" @input="onOpacityChange">
                    </div>
                    <!-- TODO: add opacity slider -->
                    <hr class="dropdown-divider" />
                    <a href="#" class="dropdown-item" @click="emit('state', '_reset', '*')">Reset</a>
                    <ColorPicker v-if="colorPickerActive" @choose="color => emit('state', 'bgColor', color)" />
                </div>
            </div>
        </div>
    </header>
    <div ref="body" :class="['card-body',stateClass,contentClass??'card-content',textColorClass,{'can-scroll': store.interactable}]" :style="contentStyle" v-if="isVisibleContent">
        <slot></slot>
        <br>
    </div>
    <div class="resize-element-container">
        <div class="resize-element" v-if="store.interactable&&store.editable" @mousedown="onResizeStart" @mouseup="onResizeStop">
            <Icon icon="fa-solid fa-up-right-and-down-left-from-center" rotate=90 />
        </div>
    </div>
</div>
</template>

<script setup lang="ts">
import { StyleValue, computed, ref } from 'vue';
import { ElemAlignment, ElemVisibility, ElementState, StateKeys, UIElement } from '@/types.ts';
import { colorToCSS, replaceVariables, shouldUseDarkText } from '@/util.ts';
import { useGlobalState } from '@/store/state.ts';
import ColorPicker from '../ColorPicker.vue'
const emit = defineEmits<{
  (e: 'state', key: StateKeys, value: any): void
}>()

const store = useGlobalState()


const props = defineProps<{
    elem: UIElement,
    state?: ElementState,
    official?: boolean

    contentClass?: string
}>()
let root = ref<HTMLElement>()
let body = ref<HTMLElement>()
let header = ref<HTMLElement>()
let dragging = ref(false)
let dropdownActive = ref(false)
let colorPickerActive = ref(false)

const bgColor = computed(() => {
    if(!store.interactable) return { r: 0, g: 0, b: 0, a: 0 }
    const color = props.state?.bgColor ?? props.elem.defaults?.bgColor ?? { r : 255, g: 255, b: 255 }
    color.a = getState("opacity", 0.6)
    return color
})
const title = computed(() => {
    return replaceVariables(getState("title", "Untitled Element"), store.variables)
})
const textColorClass = computed(() => {
    return shouldUseDarkText(bgColor.value) ? "has-text-black" : "has-text-white"
})
const visibility = computed(() => {
    return getState("visibility", ElemVisibility.Always)
})
const isVisible = computed(() => {
    if(visibility.value == ElemVisibility.Always) return true
    if(visibility.value == ElemVisibility.DisplayOnly) return !store.interactable || store.editable
    if(visibility.value == ElemVisibility.InteractableOnly) return store.interactable
    return true
})
const isVisibleContent = computed(() => {
    if(visibility.value == ElemVisibility.Always) return true
    if(visibility.value == ElemVisibility.DisplayOnly) return !store.interactable || store.editable
    if(visibility.value == ElemVisibility.InteractableOnly) return !store.interactable
    return true
})
const stateClass = computed(() => {
    if(store.editable) return "state-edit"
    if(store.interactable) return "state-interact"
    return "state-overlay"
})

function getState(key: keyof ElementState, defaultValue: any) {
    let value = props.state ? props.state[key] : undefined
    if(value == undefined) value = props.elem.defaults ? props.elem.defaults[key] : undefined
    return value ?? defaultValue
}

const size = computed(() => {
    return getState("size", { width: 300, height: 400 })
})

const position = computed(() => {
    const pos = getState("position", { x: 40, y: 40 })
    let [x,y] = [ pos.x, pos.y ]
    // if(props.elem.alignment) {
    //     switch(props.elem.alignment) {
    //         case ElemAlignment.TopRight: {
    //             console.log(pos.x, store.width, store.width - pos.x)
    //             x = store.width - pos.x
    //             // y = store.height - pos.y
    //         }
    //     }
    // }
    // if(x >= store.width) {
    //     x = store.width - size.value.width
    // }
    return { x, y }
})

const style = computed(() => {
    const xElemName = props.elem.alignment === ElemAlignment.TopLeft || props.elem.alignment === ElemAlignment.BottomLeft ? "left" : "right"
    const yElemName = props.elem.alignment === ElemAlignment.TopLeft || props.elem.alignment === ElemAlignment.TopRight ? "top" : "bottom"
    return {
        'background-color': colorToCSS(bgColor.value),
        display: dragging.value ? "hidden" : "block",
        position: "absolute",
        [xElemName]: `${position.value.x}px`,
        [yElemName]: `${position.value.y}px`,
        'z-index': props.elem.zIndex ?? 0
        // width: Math.min(size.width, document.documentElement.clientWidth - pos.x) + "px",
        // width: Math.min(size.height, document.documentElement.clientHeight - pos.y) + "px", 
        // width: size.width + "px",
        // height: size.height + "px"
        // height: props.state?.size?.height ? `${props.state?.size?.height}px` : 'inherit',
    }
})

const contentStyle = computed(() => {
    // const size = getState("size", { width: 200, height: 200 })
    return {
        'background-color': colorToCSS(bgColor.value),
        width: size.value.width + "px",
        height: size.value.height + "px"
    }
})

function toggleVisibility() {
    if(visibility.value == ElemVisibility.Always) {
        emit("state", "visibility", ElemVisibility.InteractableOnly)
    } else {
        emit("state", "visibility", ElemVisibility.Always)
    }
}

function onOpacityChange(e: any) {
    emit("state", "opacity", e.target.value / 100)
}

function onMouseMove(e: any) {
    if(!header.value) return
    const size = (header.value as HTMLElement).getBoundingClientRect()
    let x = e.clientX - (size.width/2)
    let y = e.clientY - (size.height/2)
    if(props.elem.alignment === ElemAlignment.TopRight || props.elem.alignment === ElemAlignment.BottomRight) x = document.body.clientWidth - x - size.width
    if(x < 0) x = 0
    else if(x >= document.body.clientWidth - size.width) x = document.body.clientWidth - size.width

    if(props.elem.alignment === ElemAlignment.BottomLeft || props.elem.alignment === ElemAlignment.BottomRight) y = document.body.clientHeight - y - size.height
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

function onResizeStart() {
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
        width: Math.max(distance[0], 250),
        height: Math.max(distance[1], 100)
    }
    console.log("distance", size)
    emit("state", "size", size)
    e.preventDefault();
}
function onResizeStop() {
    document.removeEventListener("mousemove", onResize)
    document.removeEventListener("mouseup", onResizeStop)
}
</script>

<style scoped>
.root {
    min-width: 11em;
    min-height: 3rem;
}
.can-scroll {
    overflow-y: auto !important;
    scrollbar-width: thin;
    scrollbar-color: grey transparent

}
.card-body {
    overflow-y: hidden;
    overflow-x: clip;
    min-width: fit-content;
    min-height: fit-content;
    padding: 8px;
    border-radius: 0;
    /* max-height: 10vh; */
}
.card {
    border-radius: 0;
}
.card-header {
    border-bottom: 2px solid rgb(151, 150, 150);
    margin-bottom: 0;
    user-select: none;
}
.card-header-title {
    padding: 8px;
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
.official {
    border: 4px dashed black;
}
.root.state-overlay {
    padding: 0;
    margin: 0;
    border: 0;
    background-color: rgba(0, 0, 0, 0)
}
.card-body.state-overlay {
    opacity: 1.0;
}
</style>