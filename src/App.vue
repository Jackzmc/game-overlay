<template>
<div :class="['container',{'interact-overlay': interactable}]">
  <div class="box rbox">
    <h1>Time: {{ time }}</h1>
    {{ interactable }}
  </div>

  <div class="box rbox procbox" v-if="proc">
    {{ JSON.stringify(proc, null, 2) }}
  </div>
</div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api'
import { emit, listen } from '@tauri-apps/api/event'
import { onMounted, onUnmounted, ref } from 'vue'
import { register, unregisterAll } from '@tauri-apps/api/globalShortcut';

let interactable = ref(false)
let time = ref()
let proc = ref()

setInterval(() => {
  const d = new Date()
  time.value = d.toLocaleTimeString()
}, 1000)

// setInterval(async () => {
//   proc.value = await invoke("check_process")
// }, 500)
onMounted(async() => {
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
  console.info("Mount done")
})
onUnmounted(async() => {
  await unregisterAll()
})
</script>

<style scoped>
.procbox {
  position: fixed;
  bottom: 0;
  right: 0;
}
</style>