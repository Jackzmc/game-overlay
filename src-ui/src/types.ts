export interface Position { x: number, y: number}
export interface Size { width: number, height: number}
export interface Color { r: number, g: number, b: number, a?: number }
export enum ElemFlags {
    None = 0,

}
// TODO: implement
export enum ElemAlignment {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight
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
    active: boolean
    type: ElemType,
    alignment?: ElemAlignment,
    zIndex?: number,
    defaults?: ElementState,
    flags?: ElemFlags,
    variables: Record<string, any>
}

export interface TextElement extends BaseElement {
    type: "text",
    template: string
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

export type ManagerResponseType = "manager_connected" | "manager_disconnected" | "joined_server" | "left_server" | "game_data" | "authorized" | "update_ui" | "register_temp_ui" 
interface ManagerResponseBase {
    type: ManagerResponseType
}
export interface ManagerResponseJoined extends ManagerResponseBase {
    type: "joined_server",
    server_id: string,
    server_name: string,
    server_ip: string
}
export interface ManagerResponseLeft extends ManagerResponseBase {
    type: "left_server",
}
export interface ManagerResponseAuthorized extends ManagerResponseBase {
    type: "authorized",
    steamid2: string,
    auth_token: string,
    user: any //SteamUser
}
export interface ManagerResponseConnected {
    type: "manager_connected"
}
export interface ManagerResponseDisconnected {
    type: "manager_disconnected"
}
export interface ManagerResponseUpdateUI {
    type: "update_ui",
    namespace: string | null,
    elem_id: string,
    visible: boolean,
    variables: Record<string, any>
}
export interface ManagerResponseRegisterTempUI {
    type: "register_temp_ui",
    elem_id: string,
    expires_seconds: number | null, 
    element: UIElement
}
export type ManagerResponse = ManagerResponseJoined | ManagerResponseLeft | ManagerResponseAuthorized | ManagerResponseDisconnected | ManagerResponseUpdateUI | ManagerResponseRegisterTempUI | ManagerResponseConnected | ManagerResponseDisconnected