import { createApp, defineAsyncComponent } from "vue";
import "./assets/main.css";
import '../node_modules/bulma/css/bulma.min.css'
import { createPinia } from 'pinia'
import App from "./App.vue";

import { library } from '@fortawesome/fontawesome-svg-core'
import { fas } from '@fortawesome/free-solid-svg-icons'
import Icon from './components/Icon.vue'


library.add(fas)


const pinia = createPinia()


createApp(App)
    .use(pinia)
    .component('Icon', Icon)
    .component('ConfirmModal', defineAsyncComponent(() =>
        import('./components/ConfirmModal.vue')
    ))
    .mount("#app");
