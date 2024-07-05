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

const COLORS_curated = [
    { "r": 132, "g": 171, "b": 15 },
    { "r": 214, "g": 157, "b": 212 },
    { "r": 213, "g": 231, "b": 248 }, 
    { "r": 140, "g": 175, "b": 105 }, 
    { "r": 244, "g": 166, "b": 22 }, 
    { "r": 50, "g": 143, "b": 39 }, 
    { "r": 238, "g": 62, "b": 251 }, 
    { "r": 107, "g": 168, "b": 120 }, 
    { "r": 46, "g": 49, "b": 25 }, 
    { "r": 134, "g": 176, "b": 135 }, 
    { "r": 12, "g": 23, "b": 235 }, 
    { "r": 53, "g": 156, "b": 219 }, 
    { "r": 127, "g": 142, "b": 218 }, 
    { "r": 237, "g": 144, "b": 248 }, 
    { "r": 231, "g": 228, "b": 211 }, 
    { "r": 25, "g": 189, "b": 174 }, 
    { "r": 139, "g": 117, "b": 89 }, 
    { "r": 20, "g": 200, "b": 118 }, 
    { "r": 167, "g": 128, "b": 193 }, 
    { "r": 25, "g": 189, "b": 174 }, 
    { "r": 233, "g": 91, "b": 112 }, 
    { "r": 169, "g": 47, "b": 95 }, 
    { "r": 235, "g": 5, "b": 166 }, 
    { "r": 17, "g": 182, "b": 97 }, 
    { "r": 203, "g": 57, "b": 15 }, 

]
const COLORS = Array(24).fill(undefined).map(() => _generateColor())

function getRandomInt(min: number, max: number) {
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