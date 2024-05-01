export interface Position { x: number, y: number}
export type ElemType = "text" | "list"

export interface BaseElement {
    id: string,
    type: ElemType,
    defaultPosition: Position,
    title?: string,
    zIndex?: number
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

export interface Action {
    label: string,
    action: string,
}

export interface ElementState {
    position?: Position
}

export type UIElement = TextElement | ListElement

const test: ListElement = {
    id: "test",
    type: "list",
    defaultPosition: { x: 0, y: 0 },
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

// function createParent(elem: BaseElement): HTMLElement {
//     const div = document.createElement("div")
//     div.id = elem.id
//     div.className = "box rbox"
//     div.style.position = "absolute"
//     div.style.left = `${elem.position.x}px`
//     div.style.top = `${elem.position.y}px`
//     return div
// }
// export function createElement(elem: UIElement): HTMLElement {
//     const root = createParent(elem)
//     // The create<type>Element functions are async, but we do not want to wait for them to be created
//     // for performance reasons.
//     switch(elem.type) {
//         case "text":
//             createTextElement(root, elem)
//             break;
//         case "list":
//             createListElement(root, elem)
//             break;
//     }
//     return root
// }

// async function createTextElement(root: HTMLElement, elem: TextElement) {
//     const span = document.createElement("span")
//     span.innerHTML = await marked.parse(elem.text)
//     root.appendChild(span)
// }

// async function createListElement(root: HTMLElement, elem: ListElement) {
//     for(const entry of elem.list) {
//         const div = document.createElement("div")
//         div.id = elem.id
//         div.className = "box rbox list-item"
//         div.innerHTML = `<await marked.parse(entry.content)
//         root.appendChild(div)
//     }
// }