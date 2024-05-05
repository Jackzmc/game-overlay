import { defineStore } from 'pinia'

export interface GlobalState {
  interactable: boolean,
  editable: boolean,
  time: string,
  date: string,
  managerConnected: boolean,
  steamid?: string,
  steamUser?: SteamUser,
  server?: {
    id: string,
    ip: string,
    name: string
  }
}

export interface SteamUser {
  steamid: string;
  communityvisibilitystate: number;
  profilestate: number;
  personaname: string;
  profileurl: string;
  avatar: string;
  avatarmedium: string;
  avatarfull: string;
  avatarhash: string;
  lastlogoff: number;
  personastate: number;
  primaryclanid: string;
  timecreated: number;
  personastateflags: number;
  loccountrycode: string;
  locstatecode: string;
}

export const useGlobalState = defineStore('state', {
    state: (): GlobalState => ({ 
        // TODO: change to view state? as this still runs, might want to stop updating if hidden
        interactable: false,
        editable: false,
        time: "",
        date: "",
        managerConnected: false,

        steamid: undefined,
        steamUser: undefined,
        server: undefined
    }),
    getters: {
      variables: (state) => {
        return {
          time: state.time,
          date: state.date,
          steamid: state.steamid ?? "[Unauthorized]",
          name: state.steamUser?.personaname ?? state.steamid ?? "Unknown",
          server: state.server?.name,
          serverip: state.server?.ip,
          avatarurl: state.steamUser?.avatarfull
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