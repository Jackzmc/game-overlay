<template>
<div class="modal is-active">
  <div class="modal-background"></div>
  <div class="modal-card">
    <header class="modal-card-head">
      <slot name="header">
        <p class="modal-card-title"><slot name="title">{{ title }}</slot></p>
        <button class="delete" aria-label="close" @click="emit('cancel')"></button>
      </slot>
    </header>
    <section class="modal-card-body">
      <slot></slot>
    </section>
    <footer class="modal-card-foot" style="display: block">
      <div class="buttons">
        <button :class="['button',type ?? 'is-danger']" @click="emit('confirm')">Confirm</button>
        <button class="button" @click="emit('cancel')">Cancel</button>
      </div>
    </footer>
  </div>
</div>
</template>
  
<script setup lang="ts">
const emit = defineEmits(["cancel", "confirm"])
const props = defineProps<{
    title: string,
    type?: "is-warning" | "is-danger" | "is-success" | "is-info"
}>()
</script>