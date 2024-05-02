import { createApp, defineAsyncComponent } from "vue";
import "./styles.css";
import '../node_modules/bulma/css/bulma.min.css'
import App from "./App.vue";


createApp(App)
    .component('ConfirmModal', defineAsyncComponent(() =>
        import('./components/ConfirmModal.vue')
    ))
    .mount("#app");
