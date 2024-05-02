import { defineStore } from 'pinia'

export const useGlobalState = defineStore('state', {
    state: () => ({ 
        interactable: false,
        editable: false,
        time: "",
        date: "",

        steamid: null,
        steamUser: null,
        serverAddr: { ip: null, port: null },
        serverName: ""
    }),
    getters: {
      variables: (state) => {
        return {
          time: state.time,
          date: state.date,
          steamid: state.steamid ?? "[Unauthorized]",
          name: state.steamUser?.personaname ?? state.steamid ?? "Unknown",
          server: state.serverName,
          serverip: `${state.serverAddr.ip}:${state.serverAddr.port}`
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