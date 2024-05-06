<template>
<div :class="{'interact-overlay': store.interactable}">
  <div ref="elementsContainer">
    <component v-for="(elem, id) in elementRegistry" :key="id" 
      :is="elem.component" 
      :elem="elem.element" 
      :state="elementStates[id]"
      @state="(key: keyof ElementState, value: any) => updateState(id, key, value)"
    />
  </div>
  <div class="toggle-edit-box">
    <div class="buttons">
      <button @click="store.editable = !store.editable" v-if="store.interactable" :class="['button',{'is-info': store.editable}]">
        <Icon icon="fa-pencil"><template #default v-if="store.editable">Edit Active</template></Icon>
      </button>
      <button @click="store.interactable = !store.interactable" class="button" v-if="!TAURI_AVAILABLE">
        {{ store.interactable ? 'Stop Interact' : 'Interact' }}
      </button>
    </div>
  </div>
  <div class="notification is-danger is-light disconnected" v-if="!store.managerConnected">
    <Icon icon="fa-exclamation-triangle ">Lost connection to server</Icon>
  </div>
</div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api'
import { listen } from '@tauri-apps/api/event'
import { markRaw, onMounted, onUnmounted, ref, } from 'vue'
import { register, unregisterAll } from '@tauri-apps/api/globalShortcut';
import { ElemType, ElemVisibility, ElementState, ManagerResponse, StateKeys, UIElement } from './types.ts';

import TextListElement from './components/elements/TextListElement.vue';
import TextElement from './components/elements/TextElement.vue';
import { ActionFlags } from './types';
import { useGlobalState } from './store/state.ts';

const TAURI_AVAILABLE = window.__TAURI_METADATA__ != undefined
const store = useGlobalState()


let proc = ref()

interface ElementData {
  component: any,
  element: UIElement
}

let elementStates = ref<Record<string, ElementState>>({})
let elementRegistry = ref<Record<string, ElementData>>({})
let elementsContainer = ref()

const ELEM_TYPE_MAP: Record<ElemType, any> = markRaw({
  "list:text": TextListElement,
  "list:dynamic": TextListElement,
  "text": TextElement,
})

// type PartialBy<T, K extends keyof T> = Omit<T, K> & Partial<Pick<T, K>>
function setElement(namespace: string | null, id: string, element: UIElement): ElementData {
  const fullId = `${namespace??''}:${id}`
  console.debug(fullId, JSON.stringify(element))
  elementRegistry.value[fullId] = {
    component: markRaw(ELEM_TYPE_MAP[element.type]),
    element: element
  }
  return elementRegistry.value[fullId] 
}
async function fetchElement(namespace: string, id: string): Promise<ElementData | undefined> {
  try {
    const elem: UIElement = await invoke("fetch_element", { namespace, id })
    return setElement(namespace, id, elem)
  } catch(err) {
    // TODO: throw better err
    alert("fetchElement:" + (err as any).message)
    return undefined
  }
}

function test() {
  setElement(null, "test", {
    type: "text",
    defaults: {
      position:{ x: 20, y: 15 },
      visibility: ElemVisibility.DisplayOnly,
      title: "Variable test",
    },
    active: true,
    text: "* Time: %time%\n* Date: %date%\n* Hello %name%, your steamid is %steamid%\n* You are on: %server% (%serverip%)",
  })
  setElement(null, "test2", {
    type: "text",
    defaults: {
      bgColor: { r: 120, g: 255, b: 255 },
      position: { x: 5, y: 250 },
      title: "test",
    },
    active: true,
    text: "# markdown test\n**blah** blah blah\nlorem ipsum dolor sit amet"
  })
  setElement(null, "big_list", {
    type: "list:text",
    defaults: {
      title: "Big List",
    },
    active: true,
    list: Array(15).fill(undefined).map((_, i) => {
      return {
        "title": `Element ${i}`,
        content: "Blah blah blah"
      }
    })
  })
  setElement(null, "list", {
    "type": "list:text",
    defaults: {
      position: { x: 400, y: 10 },
      title: "My List",
    },
    active: true,
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
  })
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
  test()
  // render()
  if(TAURI_AVAILABLE) {
    await listen("manager", ({ payload }) => {
      onManagerData(payload as ManagerResponse)
    })
    await listen("process", ({payload}) => {
      proc.value = payload
    })
    try {
      registerShortcuts()
    } catch(err) {
      // Can fail if already registered, ignore it.
    }
  } else {
    document.body.style.backgroundColor = "rebeccapurple"
    store.interactable = true
  }
  console.info("Mount done")

})

async function registerShortcuts() {
  await register('Control+home', async() => {
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
}

function clearServerUIs() {
  console.debug("Clearing server and temp UIs")
  for(const [id, data] of Object.entries(elementRegistry.value)) {
    const [namespace, elemId] = id.split(":")
    if(namespace != "global") {
      delete elementRegistry.value[id]
    }
  }
}

async function onManagerData(payload: ManagerResponse) {
  console.debug("Got payload", payload.type, payload)
  switch(payload.type) {
    case "joined_server": {
      store.server = {
        id: payload.server_id,
        name: payload.server_name,
        ip: payload.server_ip,
      }
      break;
    }
    case "left_server": {
      clearServerUIs()
      break;
    }
    case "authorized": {
      store.managerConnected = true
      store.authorize(payload.steamid2, payload.user)
      break;
    }
    case "register_temp_ui": {
      setElement(null, payload.elem_id, payload.element)
      if(payload.expires_seconds) {
        setTimeout(() => {
          delete elementRegistry.value[`:${payload.elem_id}`]
        }, 1000 * payload.expires_seconds)
      }
      break;
    }
    case "update_ui": {
      // TODO: automatically do this manager side? idk
      const id = `${payload.namespace??''}:${payload.elem_id}`
      let elem: ElementData | undefined = elementRegistry.value[id]
      if(!elem && payload.namespace) elem = await fetchElement(payload.namespace, payload.elem_id)
      if(!elem) return console.warn("No elem", payload)
      elem.element.active = payload.visibility
      break;
    }
    case "manager_connected": {
      store.managerConnected = true
      break
    }
    case "manager_disconnected": {
      store.managerConnected = false
      store.server = undefined
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
.disconnected {
  position: fixed;
  top: 0;
  right: 0;
}
</style>