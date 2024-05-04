export interface Position { x: number, y: number}
export interface Size { width: number, height: number}
export interface Color { r: number, g: number, b: number, a?: number }
export enum ElemFlags {
    None = 0,
    // Only show for survviors
    SurvivorsOnly = 1,
    // Show only on death
    ShowOnlyOnDeath = 2,
    // Show when not dead
    ShowOnlyOnAlive = 4

}
export enum ElemVisibility {
    Always = 0,
    /// Only show when overlay is in interact mode or edit mode
    InteractableOnly,
    // Only show when overlay is in non-interactable mode or edit mode
    DisplayOnly
}

export type ElemType = "text" | "list:text" | "list:dynamic" 
export interface BaseElement {
    // id: string,
    type: ElemType,
    zIndex?: number,
    defaults?: ElementState,
    flags?: ElemFlags,
}

export interface TextElement extends BaseElement {
    type: "text",
    text: string
}

export interface TextListElement extends BaseElement {
    type: "list:text",
    list: TextListElementEntry[]
}
export interface DynamicListElement extends BaseElement {
    type: "list:dynamic",
    list: Record<string, DynamicListElementEntry>,
}
export interface DynamicListElementEntry {
    title?: string,
    content: string,
    data: Record<string, string>
    actions?: Action[]
}

export interface TextListElementEntry {
    title?: string,
    content: string,
    actions?: Action[]
}

export type UIElement = TextElement | TextListElement | DynamicListElement

export enum ActionFlags {
    None,
    RequireConfirmation = 1
}
export interface Action {
    label: string,
    action: string,
    bgColor?: Color,
    flags?: ActionFlags
}

export interface ElementState {
    position?: Position,
    bgColor?: Color,
    size?: Size,
    visibility?: ElemVisibility,
    // Store separately so we can changes colors independently
    opacity?: number,
    title?: string
}

export type StateKeys = keyof ElementState | "_reset"

export type ManagerResponseType = "manager_disconnected" | "client_joined" | "client_disconnected" | "game_data" | "authorized" | "register_ui"
interface ManagerResponseBase {
    type: ManagerResponseType
}
export interface ManagerResponseAuthorized extends ManagerResponseBase {
    type: "authorized",
    steamid2: string,
    auth_token: string,
    user: any //SteamUser
}

export interface ManagerResponseDisconnected {
    type: "manager_disconnected"
}
export interface ManagerResponseRegisterUI {
    type: "register_ui",
    namespace: string,
    id: string
}

export type ManagerResponse = ManagerResponseAuthorized | ManagerResponseDisconnected | ManagerResponseRegisterUI