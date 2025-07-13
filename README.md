# Game Overlay

Adds an overlay in game that dedicated servers with a plugin can add elements to, allowing for custom elements that include text, actions, admin tools, etc. For an example, the server can send a player list, where an admin can quickly see all players, see their health, items, and any other extra information. In addition, admins could also quickly kick, ban, or perform other plugin actions, with a friendly UI than that source engine can provide.

> [!WARNING]
> Project is still in heavy development and may be abandoned for a while any time. 
> 
> In addition, as of currently, the three parts may work independently but not connected

## Implementation

All sides are written in rust, using websockets to communicate to the manager

[Client] <== websocket ==> [Manager] <== websocket ==> [Server]

## Manager (src-manager)
 The manager sits in between the servers and clients and facilates communication. It authenticates & verifies incoming requests and transmits them as events to the server/client.
 For example, when a player joins a game, the server plugin informs the manager, which checks, and then informs the players. 

 * Requires a mysql connection
 * Requires steam API key
 
## Client (src-overlay)
 The client, or the overlay, is what runs on a user's computer. When it detects the game is running, it will appear and wait for the manager telling it instructions as events.

 * Uses egui for UI
## Server (in [Jackzmc/sourcemod-plugins](https://github.com/Jackzmc/sourcemod-plugins/blob/master/scripting/sm_overlay.sp) for now)
 The server is managed as a base sourcemod plugin that controls communication with the manager and core aspects. It also incldues any additional addon plugins that hook into the main plugin and add their own custom elements or features.

 * Requires a fork of ripext with websocket support and fixes.


### Building

For both manager and client
```
cd src-manager # or src-overlay
cargo build # or run
```
