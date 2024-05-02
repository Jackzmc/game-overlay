import { createApp, defineAsyncComponent } from "vue";
import "./styles.css";
import '../node_modules/bulma/css/bulma.min.css'
import { createPinia } from 'pinia'
import App from "./App.vue";

const pinia = createPinia()


createApp(App)
    .use(pinia)
    .component('ConfirmModal', defineAsyncComponent(() =>
        import('./components/ConfirmModal.vue')
    ))
    .mount("#app");
