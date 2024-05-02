<template>
<div class="card root" ref="root" :style="style as StyleValue" @click="endDrag" @mouseleave="dropdownActive = false">
    <header ref="header" class="card-header" :style="{cursor: editable?'move':'inherit'}" v-if="elem.title" @click="startDrag(false)" >
        <p :class="['card-header-title',textColorClass]">
            {{ elem.title }}
        </p>
        <div v-if="interactable" :class="['card-header-icon','dropdown',{'is-active': dropdownActive}]">
            <div class="dropdown-trigger"   @click.prevent="dropdownActive = !dropdownActive">
                <button class="" aria-haspopup="true" aria-controls="dropdown-menu">
                <span class="icon is-small">
                    <i class="fas fa-angle-down" aria-hidden="true">...</i>
                </span>
                </button>
            </div>
            <div class="dropdown-menu"role="menu">
                <div class="dropdown-content" @click="dropdownActive = false">
                    <a class="dropdown-item" @click="startDrag(true)">Move</a>
                    <a class="dropdown-item" @click="contentVisible = !contentVisible">{{ contentVisible ? 'Hide Content' : 'Show Content' }}</a>
                    <!-- <a class="dropdown-item">Change Color</a> -->
                <!-- <hr class="dropdown-divider" />
                <a href="#" class="dropdown-item"> With a divider </a> -->
                </div>
            </div>
        </div>
    </header>
    <div :class="[contentClass??'card-content',textColorClass]" :style="contentStyle" v-if="contentVisible">
        <slot></slot>
    </div>
</div>
</template>

<script setup lang="ts">
import { StyleValue, computed, onMounted, ref } from 'vue';
import { ElementState, UIElement } from '../../types.ts';
import { colorToCSS, shouldUseDarkText } from '../../util.ts';
const emit = defineEmits<{
  (e: 'pos', x: number, y: number): void
}>()


const props = defineProps<{
    elem: UIElement,
    state?: ElementState,
    editable?: boolean,
    interactable?: boolean

    contentClass?: string
}>()
let root = ref<HTMLElement>()
let header = ref<HTMLElement>()
let contentVisible = ref(true)
let dragging = ref(false)
let dropdownActive = ref(false)

const bgColor = computed(() => {
    return props.state?.bgColor ?? props.elem.defaults?.bgColor ?? { r : 255, g: 255, b: 255, a: 1 }
})
const textColorClass = computed(() => {
    return shouldUseDarkText(bgColor.value) ? "has-text-black" : "has-text-white"
})

const style = computed(() => {
    return {
        'background-color': colorToCSS(bgColor.value),
        display: dragging.value ? "hidden" : "block",
        position: "absolute",
        left: `${props.state?.position?.x ?? props.elem.defaults?.position?.x ?? 0}px`,
        top: `${props.state?.position?.y ?? props.elem.defaults?.position?.y ?? 0}px`
    }
})

const contentStyle = computed(() => {
    return {
        'background-color': colorToCSS(bgColor.value),
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
        dropdownActive.value = false
        console.debug("startDrag")
        dragging.value = true
        dragStart = Date.now()
        window.addEventListener("mousemove", onMouseMove)
    }
} 
function endDrag() {
    if(!dragStart) return
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