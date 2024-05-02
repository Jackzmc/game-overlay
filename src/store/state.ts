import { defineStore } from 'pinia'

export const useGlobalState = defineStore('state', {
    state: () => ({ 
        interactable: false,
        editable: false,
        time: "",
        date: "",

        steamid: "",
        steamUser: ""
    }),
    getters: {
      variables: (state) => {
        return {
          time: state.time,
          date: state.date
        }
      }
    },
    actions: {
      updateTime() {
        const d = new Date()
        this.time = d.toLocaleTimeString()
        this.date = d.toLocaleDateString()
      },
      authorize(steamid: string, user: any) {
        this.steamid = steamid
        this.steamUser = user
      }
    },
})