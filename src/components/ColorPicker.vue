<template>
<div class="table-container">
    <table class="table">
        <SplitTable :items-per-column="6" :data="COLORS" v-slot="slotProps">
            <tr>
                <td v-for="item in slotProps.data" :key="item" 
                    @click="emit('choose', item)" class="bd-color-swatch" 
                    :style="'background-color: ' + colorToCSS(item)">
                </td>
            </tr>
        </SplitTable>
    </table>
</div>
</template>

<script setup lang="ts">
import SplitTable from './SplitTable.vue';
import { colorToCSS } from '../util.ts'

const emit = defineEmits(["choose"])

const COLORS = Array(24).fill(undefined).map(() => _generateColor())

function getRandomInt(min, max) {
  const minCeiled = Math.ceil(min);
  const maxFloored = Math.floor(max);
  return Math.floor(Math.random() * (maxFloored - minCeiled) + minCeiled); // The maximum is exclusive and the minimum is inclusive
}

function _generateColor() {
    return { r: getRandomInt(0, 255), g: getRandomInt(0, 255), b: getRandomInt(0, 255) }
}
</script>

<style scoped>
.table {
    border-collapse: separate; 
    border-spacing: 5px;
}
</style>