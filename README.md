# Game Overlay

A in-game overlay for source engine games that allow for custom UI elements, such as text, graphics, admin tools, etc, driven by server side plugins.
Servers with overlay plugin can send custom elements, such as an admin tool to quickly add notes to any players with a friendlier UI than in-game source engine menus.

Other examples are elements that show extra player information such as health, items, position, and any other information fed from custom plugins. 

> [!WARNING]
> Project is still in heavy development and may be abandoned for a while any time. 
> 
> In addition, as of currently, the three parts may work independently but not connected

## Implementation

The manager and client are written in rust, using websockets to communicate to the manager.
The server plugin is written in as a sourcemod plugin.

[Client] <== websocket ==> [Manager] <== websocket ==> [Server]

## Manager (in [manager](./manager))
 The manager sits in between the servers and clients and facilates communication. It authenticates & verifies incoming requests and transmits them as events to the server/client.
 For example, when a player joins a game, the server plugin informs the manager, which checks, and then informs the players. 

 * Requires a mysql connection
 * Requires steam API key
 
## Client (in [client](./client))
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
