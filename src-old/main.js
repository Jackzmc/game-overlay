const { invoke } = window.__TAURI__.tauri;

let greetInputEl;
let greetMsgEl;

async function greet() {
  // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
  greetMsgEl.textContent = await invoke("greet", { name: greetInputEl.value });
}

window.addEventListener("DOMContentLoaded", () => {
  greetInputEl = document.querySelector("#greet-input");
  greetMsgEl = document.querySelector("#greet-msg");
  document.querySelector("#greet-form").addEventListener("submit", (e) => {
    e.preventDefault();
    greet();
  });
});

const t = document.querySelector("#time")
setInterval(() => {
  const d = new Date()
  t.textContent = d.toLocaleTimeString()

}, 1000)

setInterval(async () => {
  const pid = await invoke("check_process")
  console.log("pid", pid)
}, 500)

