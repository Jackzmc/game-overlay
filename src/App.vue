<template>
<div :class="{'interact-overlay': store.interactable}">
  <div class="toggle-edit-box">
    <div class="buttons">
      <button @click="store.editable = !store.editable" v-if="store.interactable" :class="['button',{'is-info': store.editable}]">
        <Icon icon="fa-pencil"><template #default v-if="store.editable">Edit Active</template></Icon>
      </button>
      <button @click="store.interactable = !store.interactable" class="button">
        {{ store.interactable ? 'Stop Interact' : 'Interact' }}
      </button>
    </div>
  </div>
  <div ref="elementsContainer">
    <component v-for="(elem, id) in elementRegistry" :key="id" 
      :is="elem.component" 
      :elem="elem.element" 
      :state="elementStates[id]"
      @state="(key: keyof ElementState, value: any) => updateState(id, key, value)"
    />
  </div>

</div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api'
import { emit, listen } from '@tauri-apps/api/event'
import { inject, markRaw, onMounted, onUnmounted, provide, ref, shallowRef } from 'vue'
import { register, unregisterAll } from '@tauri-apps/api/globalShortcut';
import { ElemType, ElemVisibility, ElementState, ManagerResponse, StateKeys, UIElement } from './types.ts';

import ListElement from './components/elements/ListElement.vue';
import TextElement from './components/elements/TextElement.vue';
import { ActionFlags } from './types';
import { useGlobalState } from './store/state.ts';

const TAURI_AVAILABLE = window.__TAURI_METADATA__ != undefined
const store = useGlobalState()


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
    title: "Variable test",
    text: "* Time: %time%\n* Date: %date%\n* Hello %name%, your steamid is %steamid%\n* You are on: %server% (%serverip%)",
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
  elements["big_list"] = {
    id: "big_list",
    type: "list",
    title: "Big List",
    list: Array(15).fill(undefined).map((_, i) => {
      return {
        "title": `Element ${i}`,
        content: "Blah blah blah"
      }
    })
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

function updateState(id: string, key: StateKeys, value: any) {
  if(key === "_reset") {
    if(value == "*") elementStates.value[id] = {}
    else delete elementStates.value[id][value as keyof ElementState]
  } else {
    if(!elementStates.value[id]) elementStates.value[id] = {}
    elementStates.value[id][key] = value
  }
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
      store.interactable = await invoke("overlay_key")
      const r: HTMLElement = document.querySelector(':root')!;
      if(store.interactable) {
        document.body.classList.add("interact-overlay")
        r.style.setProperty("--opacity", "1.0")
      } else {
        store.editable = false
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