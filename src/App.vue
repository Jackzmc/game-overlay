<template>
<div>
  <div :class="['container',{'interact-overlay': interactable}]">
    <div class="box rbox">
      <h1>Time: {{ time }}</h1>
      {{ interactable }}
    </div>

    <div class="box rbox procbox" v-if="proc">
      {{ JSON.stringify(proc, null, 2) }}
    </div>
  </div>
  <div ref="elementsContainer">
    <component v-for="(elem, id) in elementRegistry" :key="id" 
      :is="elem.component" 
      :elem="elem.element" 
      :state="elementStates[id]"
      @pos="(x: number, y: number) => updatePos(id, x, y)"
    />
  </div>
</div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api'
import { emit, listen } from '@tauri-apps/api/event'
import { markRaw, onMounted, onUnmounted, ref, shallowRef } from 'vue'
import { register, unregisterAll } from '@tauri-apps/api/globalShortcut';
import { ElemType, ElementState, UIElement } from './types.ts';

import ListElement from './components/elements/ListElement.vue';
import TextElement from './components/elements/TextElement.vue';

const TAURI_AVAILABLE = window.__TAURI_METADATA__ != undefined


let interactable = ref(false)
let time = ref()
let proc = ref()

interface ElementData {
  component: any,
  element: UIElement
}

let elementStates = ref<Record<string, ElementState>>({})
let elementRegistry = ref<Record<string, ElementData>>({})
let elementsContainer = ref()
// async function render() {
//   const ordered = Object.values(elements.value).sort((a, b) => (b.zIndex??0) - (a.zIndex??0))
//   for(const elem of ordered) {
//     const div = createElement(elem)
//     elementsContainer.value.appendChild(div)
//   }
// }

const ELEM_TYPE_MAP: Record<ElemType, any> = markRaw({
  "list": ListElement,
  "text": TextElement,
})

function test() {
  const elements: Record<string, UIElement> = {}
  elements["test"] = {
    id: "test",
    type: "text",
    defaultPosition: { x: 20, y: 15 },
    title: "test",
    text: "blah blah blah blah blah"
  }
  elements["test2"] = {
    id: "test",
    type: "text",
    defaultPosition: { x: 5, y: 250 },
    title: "test",
    text: "# markdown test\n**blah** blah blah\nlorem ipsum dolor sit amet"
  }
  elements["list"] = {
    "id": "list",
    "type": "list",
    "defaultPosition": { x: 400, y: 10 },
    title: "My List",
    list: [
      {
        "title": "Element 1",
        content: "Blah blah blah"
      },
      {
        "title": "Element 2",
        content: "Blah blah blah"
      },
      {
        "title": "Element 3",
        content: "Blah blah blah. But longer. Blah blah."
      }
    ]
  }
  return elements
}

function updatePos(id: string, x: number, y: number) {
  if(!elementStates.value[id]) elementStates.value[id] = {}
  elementStates.value[id].position = { x, y }
  saveElements()
}

setInterval(() => {
  const d = new Date()
  time.value = d.toLocaleTimeString()
}, 1000)


function saveElements() {
  localStorage.setItem("elem_data", JSON.stringify(elementStates.value))
}

function loadElements() {
  const data = localStorage.getItem("elem_data")
  if(data) {
    elementStates.value = JSON.parse(data)
  }
}

onMounted(async() => {
  loadElements()
  const elems = test()
  for(const [id, elem] of Object.entries(elems)) {
    elementRegistry.value[id] = {
      component: markRaw(ELEM_TYPE_MAP[elem.type]),
      element: elem
    }
  }
  // render()
  if(TAURI_AVAILABLE) {
    await listen("manager", ({ payload }) => {
      if(payload == "ManagerDisconnected") {

      }
      console.debug("manager", payload)
    })
    await listen("process", ({payload}) => {
      proc.value = payload
    })
    await register('Control+Shift+G', async() => {
      interactable.value = await invoke("overlay_key")
      const r: HTMLElement = document.querySelector(':root')!;
      if(interactable.value) {
        document.body.classList.add("interact-overlay")
        r.style.setProperty("--opacity", "1.0")
      } else {
        document.body.classList.remove("interact-overlay")
        r.style.setProperty("--opacity", "0.5")
      }
    });
  } else {
    document.body.style.backgroundColor = "rebeccapurple"
  }
  console.info("Mount done")

})
onUnmounted(async() => {
  // saveElements()
  if(TAURI_AVAILABLE) {
    await unregisterAll()
  }
})
</script>

<style scoped>
.procbox {
  position: fixed;
  bottom: 0;
  right: 0;
}
</style>