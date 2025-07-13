# Game Overlay

Adds an overlay that modded game servers can interact with, allowing for custom text and actions for players. Examples are button actions for moderators (kick, ban, mute), or informational (extra player's names and health).

> [!WARNING]
> This project is under heavy development. Currently, all 3 parts work can work independently but have not been tested together.

## Implementation

There are 3 parts:

* Manager
  * Management UI for Server Operators (defining UI elements)
* Client
  * Overlay UI (the actual overlay)
* Server
  * Plugin API that sends/receives
  * Plugin(s) implementing API that send and handle data

In practice, the manager is an inbetween party that facilates communication between clients and servers. Both the server and clients have a persistent websocket connection to the manager. 
The server (via the Server Plugin) communicates to the manager, sending commands such as `UpdateUI` or `PlayerJoined`, specifiying steamid(s). The manager then forwards these messages to the specified clients if they are connected. The client then receives them from the manager and processes them. This can also work in reverse, with the client sending actions (buttons they have pressed).

### Manager

Written in rust, in `src-manager`. Run with `cargo run`

### Client

Requires the UI dev server to be running in `src-ui`, run with `yarn serve` (install dependencies first with `yarn`)
Written in rust with tauri, in `src-tauri`. Run with `cargo run`.

### Server

See `src-server/sourcemod` for the sourcemod plugin. Requires a fork of ripext with websocket support and fixes.
