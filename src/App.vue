<template>
<div>
  <div :class="['container',{'interact-overlay': interactable}]">
    <div class="toggle-edit-box">
      <div class="buttons">
        <button @click="interactable = !interactable">
          {{ interactable ? 'Stop Interact' : 'Interact' }}
        </button>
        <button @click="editable = !editable" v-if="interactable">
          {{ editable ? 'Stop Move' : 'Move Elements' }}
        </button>
      </div>
    </div>
  </div>
  <div ref="elementsContainer">
    <component v-for="(elem, id) in elementRegistry" :key="id" 
      :is="elem.component" 
      :elem="elem.element" 
      :state="elementStates[id]"
      :editable="editable"
      :interactable="interactable"
      @pos="(x: number, y: number) => updatePos(id, x, y)"
    />
  </div>
</div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api'
import { emit, listen } from '@tauri-apps/api/event'
import { inject, markRaw, onMounted, onUnmounted, provide, ref, shallowRef } from 'vue'
import { register, unregisterAll } from '@tauri-apps/api/globalShortcut';
import { ElemType, ElemVisibility, ElementState, ManagerResponse, UIElement } from './types.ts';

import ListElement from './components/elements/ListElement.vue';
import TextElement from './components/elements/TextElement.vue';
import { ActionFlags } from './types';
import { useGlobalState } from './store/state.ts';

const TAURI_AVAILABLE = window.__TAURI_METADATA__ != undefined
const store = useGlobalState()


let interactable = ref(false)
let editable = ref(false)
let time = ref()
let proc = ref()

interface ElementData {
  component: any,
  element: UIElement
}

// TODO: add vuex/pinia, for global variable injections

let elementStates = ref<Record<string, ElementState>>({})
let elementRegistry = ref<Record<string, ElementData>>({})
let elementsContainer = ref()

const ELEM_TYPE_MAP: Record<ElemType, any> = markRaw({
  "list": ListElement,
  "text": TextElement,
})

function test() {
  const elements: Record<string, UIElement> = {}
  elements["test"] = {
    id: "test",
    type: "text",
    defaults: {
      position:{ x: 20, y: 15 },
    },
    title: "test",
    text: "blah blah blah blah blah. hi the time is %time%",
    visibility: ElemVisibility.DisplayOnly
  }
  elements["test2"] = {
    id: "test",
    type: "text",
    defaults: {
      bgColor: { r: 120, g: 255, b: 255 },
      position: { x: 5, y: 250 },
    },
    title: "test",
    text: "# markdown test\n**blah** blah blah\nlorem ipsum dolor sit amet"
  }
  elements["list"] = {
    "id": "list",
    "type": "list",
    defaults: {
      position: { x: 400, y: 10 }
    },
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
        content: "Blah blah blah. But longer. Blah blah.",
        actions: [
          {
            "label": "Kick Player",
            "action": "kick #5235"
          },
          {
            "label": "Ban Player",
            "action": "ban #5235",
            "bgColor": { r: 255, g: 120, b: 50, a: 1 },
            flags: ActionFlags.RequireConfirmation
          },
          {
            "label": "Add Note",
            "action": "note #STEAM_##"
          }
        ]
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
  store.updateTime()
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
      onManagerData(payload as ManagerResponse)
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

function onManagerData(payload: ManagerResponse) {
  switch(payload.type) {
    case "authorized": {
      store.authorize(payload.steamid2, payload.user)
      break;
    }
  }
}
onUnmounted(async() => {
  // saveElements()
  if(TAURI_AVAILABLE) {
    await unregisterAll()
  }
})
</script>

<style scoped>
.toggle-edit-box {
  position: fixed;
  background-color: white;
  border: 1px solid black;
  top: 0;
  left: 0;
  padding: 10px;
}
.procbox {
  position: fixed;
  bottom: 0;
  right: 0;
}
</style>