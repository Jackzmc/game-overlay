<template>
<div :class="{'interact-overlay': store.interactable}">
  <div ref="elementsContainer">
    <component v-for="(instRegistry, id) in instances" :key="instRegistry.instanceId" 
      :is="instRegistry.templateRegistry.component" 
      :id="instRegistry.instanceId"
      :instance="instRegistry.instance" 
      :instance-template="instRegistry.templateRegistry.template"
      :official="instRegistry.templateId.startsWith('overlay')"
      @state="( key: keyof ElementState, value: any ) => updateInstance( id, key, value )"
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
import { ElemAlignment, ElemType, ElemVisibility, ElementInstance, ElementState, ElementTemplate, ManagerResponse, StateKeys } from './types.ts';
import { useGlobalState } from './store/state.ts';
import { computed } from '@vue/reactivity';

import TextListElement from './components/elements/TextListElement.vue';
import TextElement from './components/elements/TextElement.vue';
import { v4 as uuidv4 } from 'uuid'

const TAURI_AVAILABLE = window.__TAURI_METADATA__ != undefined
const store = useGlobalState()


let proc = ref()

export interface TemplateRegistryData {
  id: string,
  component: any,
  template: ElementTemplate
}
export interface InstanceRegistryData {
  templateId: string,
  templateRegistry: TemplateRegistryData,

  instanceId: string,
  instance: ElementInstance
}

let instances = ref<Record<string, InstanceRegistryData>>( {} )
let templateRegistry = new Map<string, TemplateRegistryData>()

// let elementStates = ref<Record<string, ElementState>>({})
// let elementRegistry = ref<Record<string, ElementData>>( {} )
let elementsContainer = ref() // html ref
let trustedServerIds = ref<Record<string, boolean>>({})

const isServerTrusted = computed(() => {
  if(!store.server) return undefined
  return trustedServerIds.value[store.server.id]
})

const ELEM_TYPE_MAP: Record<ElemType, any> = markRaw({
  "list:text": TextListElement,
  "list:dynamic": TextListElement,
  "text": TextElement,
})

function registerTemplate( namespace: string, templateId: string, template: ElementTemplate ) {
  const fullId = `${namespace ?? ''}:${templateId}`
  console.debug( 'registerTemplate', fullId, JSON.stringify( template ) )
  if ( template.alignment == undefined ) template.alignment = ElemAlignment.TopLeft
  templateRegistry.set( fullId, {
    id: fullId,
    component: markRaw( ELEM_TYPE_MAP[template.type] ),
    template: template
  })
  return templateRegistry.get(fullId)
}

function createElement( templateId: string, data: ElementInstance ): InstanceRegistryData | undefined {
  if(!templateId) throw new Error("Invalid template ID: " + templateId)
  const templateData = templateRegistry.get( templateId )
  if ( !templateData ) throw new Error("Unknown template: " + templateId)
  const instanceId = uuidv4()
  const instance: InstanceRegistryData = {
    templateId,
    templateRegistry: markRaw(templateData),

    instanceId,
    instance: {
      variables: data.variables,
      state: {
        position: data.state.position,
        bgColor: data.state.bgColor,
        size: data.state.size,
        visibility: data.state.visibility,
        // Store separately so we can changes colors independently
        opacity: data.state.opacity,
        title: data.state.title,
      }
    }
  }
  console.debug("createElement", instanceId, "of template", templateId)
  instances.value[instanceId] = instance
  return instance
}

// type PartialBy<T, K extends keyof T> = Omit<T, K> & Partial<Pick<T, K>>
// function setElement(namespace: string | null, id: string, element: UIElement): ElementData {
//   const fullId = `${namespace??''}:${id}`
//   console.debug(fullId, JSON.stringify(element))
//   if(element.alignment == undefined) element.alignment = ElemAlignment.TopLeft
//   elementRegistry.value[fullId] = {
//     id: fullId,
//     component: markRaw(ELEM_TYPE_MAP[element.type]),
//     element: element
//   }
//   return elementRegistry.value[fullId] 
// }
async function fetchElementTemplate(namespace: string, id: string): Promise<TemplateRegistryData | undefined> {
  try {
    const elem: ElementTemplate = await invoke("fetch_template", { namespace, id })
    return registerTemplate(namespace, id, elem)
  } catch(err) {
    // TODO: throw better err
    alert("fetchElementTemplate:" + (err as any).message)
    return undefined
  }
}

function test() {
  /*
  setElement("system", "trust_server", {
    type: "text",
    defaults: {
      position: { x: 20, y: 20 },
      opacity: 1.0,
      visibility: ElemVisibility.DisplayOnly,
      title: "Server Not Trusted",
      bgColor: { r: 255, g: 172, b: 66 }
    },
    variables: {},
    zIndex: 10,
    active: true,
    template: "<div class='has-text-black has-text-centered'><h1>Trust Server</h1><p>You are connected to <b> {{ server.name }}</b> ({{ server.ip }}) for the first time. No elements will be loaded until you trust this server.</p><br><p>Do you trust this server?</p><br><div class='buttons is-centered'><div class='button is-info'>Trust Server</div><div class='button'>Dismiss</div></div></div>",
  })
  setElement(null, "player_note", {
    type: "text",
    defaults: {
      position:{ x: 20, y: 15 },
      title: "Player Notes",

    },
    alignment: ElemAlignment.TopRight,
    variables: {
      steamid: "STEAM_",
      name: "Disgruntled Pea",
      "notes":[{"id":2425,"content":"his ass crack is the lube dispenser","client":{"id":"STEAM_1:1:10882645","name":"Disgruntled Pea","avatarUrl":"https://admin.jackz.me/api/users/STEAM_1:1:10882645/avatar","isAdmin":false},"markedBy":{"id":"STEAM_1:0:530680608","name":"Ashley Golix❤","avatarUrl":"https://admin.jackz.me/api/users/STEAM_1:0:530680608/avatar"},"banned":false,"action":null},{"id":2424,"content":"stores lube in his ass crack lol","client":{"id":"STEAM_1:1:10882645","name":"Disgruntled Pea","avatarUrl":"https://admin.jackz.me/api/users/STEAM_1:1:10882645/avatar","isAdmin":false},"markedBy":{"id":"STEAM_1:1:421048382","name":"Mello Yello","avatarUrl":"https://admin.jackz.me/api/users/STEAM_1:1:421048382/avatar"},"banned":false,"action":null},{"id":2415,"content":"identifies as a rat","client":{"id":"STEAM_1:1:10882645","name":"Disgruntled Pea","avatarUrl":"https://admin.jackz.me/api/users/STEAM_1:1:10882645/avatar","isAdmin":false},"markedBy":{"id":"STEAM_1:0:530680608","name":"Ashley Golix❤","avatarUrl":"https://admin.jackz.me/api/users/STEAM_1:0:530680608/avatar"},"banned":false,"action":null},{"id":2405,"content":"bube","client":{"id":"STEAM_1:1:10882645","name":"Disgruntled Pea","avatarUrl":"https://admin.jackz.me/api/users/STEAM_1:1:10882645/avatar","isAdmin":false},"markedBy":{"id":"STEAM_1:0:204230496","name":"Liquor","avatarUrl":"https://admin.jackz.me/api/users/STEAM_1:0:204230496/avatar"},"banned":false,"action":null},{"id":2067,"content":"a cheap whore","client":{"id":"STEAM_1:1:10882645","name":"Disgruntled Pea","avatarUrl":"https://admin.jackz.me/api/users/STEAM_1:1:10882645/avatar","isAdmin":false},"markedBy":{"id":"STEAM_1:0:530680608","name":"Ashley Golix❤","avatarUrl":"https://admin.jackz.me/api/users/STEAM_1:0:530680608/avatar"},"banned":false,"action":null},{"id":449,"content":" got herpes from a jockey","client":{"id":"STEAM_1:1:10882645","name":"Disgruntled Pea","avatarUrl":"https://admin.jackz.me/api/users/STEAM_1:1:10882645/avatar","isAdmin":false},"markedBy":{"id":"STEAM_1:0:530680608","name":"Ashley Golix❤","avatarUrl":"https://admin.jackz.me/api/users/STEAM_1:0:530680608/avatar"},"banned":false,"action":null},{"id":438,"content":"has a pet rat that lives inside his anus.","client":{"id":"STEAM_1:1:10882645","name":"Disgruntled Pea","avatarUrl":"https://admin.jackz.me/api/users/STEAM_1:1:10882645/avatar","isAdmin":false},"markedBy":{"id":"STEAM_1:0:530680608","name":"Ashley Golix❤","avatarUrl":"https://admin.jackz.me/api/users/STEAM_1:0:530680608/avatar"},"banned":false,"action":null},{"id":244,"content":"if he leaves it means he sharted himself","client":{"id":"STEAM_1:1:10882645","name":"Disgruntled Pea","avatarUrl":"https://admin.jackz.me/api/users/STEAM_1:1:10882645/avatar","isAdmin":false},"markedBy":{"id":"STEAM_1:0:530680608","name":"Ashley Golix❤","avatarUrl":"https://admin.jackz.me/api/users/STEAM_1:0:530680608/avatar"},"banned":false,"action":null}]
    },
    active: true,
    template: "<h4>Notes for {{name}}</h4>{{#each notes}}<div class='list-item'><b>{{this.markedBy.name}}</b><p>{{this.content}}</p></div>{{/each}}",
  })
  setElement(null, "test2", {
    type: "text",
    defaults: {
      bgColor: { r: 120, g: 255, b: 255 },
      position: { x: 200, y: 250 },
      title: "test",
    },
    variables: {},
    active: true,
    template: "<a href='https://google.com'>Malicious Link</a>&nbsp;&nbsp;<a href='javascript:alert(1)'>Alert</a> <div class='box'>test</div> <img src='https://cdn.jackz.me/img/steve.png' />"
  } )
  setElement( null, "server", {
    type: "text",
    defaults: {
      bgColor: { r: 120, g: 255, b: 255 },
      position: { x: 600, y: 550 },
      title: "server",
    },
    variables: {},
    active: true,
    template: "%server_ip% %server_id% %steamid% %name%"
  } )
  setElement(null, "big_list", {
    type: "list:text",
    defaults: {
      title: "Big List",
    },
    variables: {},
    active: true,
    list: Array(15).fill(undefined).map((_, i) => {
      return {
        "title": `Element ${i}`,
        content: "Blah blah blah"
      }
    })
  } )
    */
  registerTemplate( "test", "player_list", {
    type: "text",
    defaults: {
      title: "Players"
    },
    // variables: {
    //   players: [
    //     {
    //       userid: 134,
    //       name: "Player 1",
    //       steamid: "STEAM_#####"
    //     },
    //     {
    //       userid: 16346,
    //       name: "Player 2",
    //       steamid: "STEAM_#####"
    //     },
    //     {
    //       userid: 134,
    //       name: "Player 3",
    //       steamid: "STEAM_#####"
    //     },
    //   ]
    // },
    hbTemplate: `
      {{#if interactable}}
        <div class="list">
        {{#each players}}
        <div class="list-item">
          <div class="list-item-content">
            <div class="list-item-title">
              {{ this.name }}
            </div>
            <div class="list-item-description">
              <span class="tag is-black">{{ this.steamid }}</span>
            </div>
          </div>
          <div class="list-item-controls has-visible-pointer-controls">
            <div class="buttons is-right">
              <button class="button" data-action="players:{{ this.steamid }}">
                <span class="icon is-small">
                  <i class="fas fa-ellipsis"></i>
                </span>
                <span>Open</span>
              </button>
            </div>
          </div>
        </div>
        {{/each}}
        </div>
      {{/if}}`
  } )
  createElement( "test:player_list", {
    variables: {
      players: [
        {
          userid: 134,
          name: "Player 1",
          steamid: "STEAM_#####"
        },
        {
          userid: 16346,
          name: "Player 2",
          steamid: "STEAM_#####"
        },
        {
          userid: 134,
          name: "Player 3",
          steamid: "STEAM_#####"
        },
      ]
    },
    state: {}
  })
  registerTemplate( "test", "player_info", {
    type: "text",
    defaults: {
      title: "Players"
    },
    // variables: {
    //   player: {
    //     userid: 134,
    //     name: "Player 3",
    //     steamid: "STEAM_#####"
    //   },
    // },
    hbTemplate: `
      {{#if interactable}}
        {{ player }}
      {{/if}}`
  } )
  // setElement(null, "list", {
  //   "type": "list:text",
  //   defaults: {
  //     position: { x: 400, y: 10 },
  //     title: "My List",
  //   },
  //   variables: {},
  //   active: true,
  //   list: [
  //     {
  //       "title": "Element 1",
  //       content: "Blah blah blah"
  //     },
  //     {
  //       "title": "Element 2",
  //       content: "Blah blah blah"
  //     },
  //     {
  //       "title": "Element 3",
  //       content: "Blah blah blah. But longer. Blah blah.",
  //       actions: [
  //         {
  //           "label": "Kick Player",
  //           "action": "kick #5235"
  //         },
  //         {
  //           "label": "Ban Player",
  //           "action": "ban #5235",
  //           "bgColor": { r: 255, g: 120, b: 50, a: 1 },
  //           flags: ActionFlags.RequireConfirmation
  //         },
  //         {
  //           "label": "Add Note",
  //           "action": "note #STEAM_##"
  //         }
  //       ]
  //     }
  //   ]
  // })
}

// TODO: re-implement
function updateInstance( id: string, key: StateKeys, value: any ) {
  const { instance } = instances.value[id]
  if(key === "_reset") {
    if(value == "*") instance.state = {}
    else delete instance.state[value as keyof ElementState]
  } else {
    if ( !instance.state ) instance.state = {}
    instance.state[key] = value
  }
  saveData()
}

setInterval(() => {
  store.updateTime()
}, 1000)


function saveData() {
  localStorage.setItem("instance_data", JSON.stringify(instances.value))
  localStorage.setItem("trusted_servers", JSON.stringify(trustedServerIds.value))
}

function loadData() {
  let data = localStorage.getItem("instance_data")
  if(data)
    instances.value = JSON.parse(data)
  data = localStorage.getItem("trusted_servers")
  if(data)
    trustedServerIds.value = JSON.parse(data)
}

function onWindowResize() {
  store.height = document.documentElement.clientHeight
  store.width = document.documentElement.clientWidth
}

onMounted(async() => {
  addEventListener("resize", onWindowResize)

  loadData()
  test()
  // render()
  const query = new URLSearchParams(window.location.search)
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
    store.interactable = (query.get("interact") ?? 1) == 1
    store.managerConnected = true
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
  for(const [id, data] of Object.entries(instances.value)) {
    const [namespace, elemId] = id.split(":")
    if(namespace != "global") {
      delete instances.value[id]
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
      // TODO: re-implement
      // setElement(null, payload.elem_id, payload.element)
      // // if(payload.expires_seconds) {
      // //   setTimeout(() => {
      // //     delete elementRegistry.value[`:${payload.elem_id}`]
      // //   }, 1000 * payload.expires_seconds)
      // // }
      break;
    }
    case "update_ui": {
      // TODO: re-implement
      // const id = `${payload.namespace??''}:${payload.elem_id}`
      // let elem: ElementData | undefined = elementRegistry.value[id]
      // if(!elem && payload.namespace) elem = await fetchElementTemplate(payload.namespace, payload.elem_id)
      // if(!elem) return console.warn("No elem", payload)
      // elem.element.active = payload.visible
      // elem.element.variables = payload.variables
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
  removeEventListener("resize", onWindowResize)
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

<style src="../node_modules/bulma-list/css/bulma-list.css"></style>