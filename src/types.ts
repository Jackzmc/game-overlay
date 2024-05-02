export interface Position { x: number, y: number}
export interface Color { r: number, g: number, b: number, a?: number }
export type ElemType = "text" | "list"
export enum ElemFlags {
    None = 0,
}
export enum ElemVisibility {
    Always = 0,
    /// Only show when overlay is in interact mode
    InteractableOnly,
    // Only show when overlay is in non-interactable mode
    DisplayOnly
}

export interface BaseElement {
    id: string,
    type: ElemType,
    title?: string,
    zIndex?: number,
    defaults?: ElementState,
    flags?: ElemFlags,
    visibility?: ElemVisibility
}

export interface TextElement extends BaseElement {
    type: "text",
    text: string
}

export interface ListElement extends BaseElement {
    type: "list",
    list: ListElementEntry[]
}
export interface ListElementEntry {
    title?: string,
    content: string,
    actions?: Action[]
}

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
    bgColor?: Color
}

export type UIElement = TextElement | ListElement

const test: ListElement = {
    id: "test",
    type: "list",
    defaults: {
        position: { x: 0, y: 0 }
    },
    title: "Players",
    list: [
        {
            "content": "Player 1",
            "actions": [
                {
                    "label": "Kick",
                    "action": "kick #1235"
                }
            ]
        }
    ]
}