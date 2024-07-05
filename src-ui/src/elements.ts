export interface Position { x: number, y: number}
class BaseElement {
    #id: string
    #title?: string
    #zIndex: number
    #position: Position

    constructor(id: string, position: Position, title?: string, zIndex?: number) {
        this.#id = id
        this.#position = position
        this.#title = title
        this.#zIndex = zIndex ?? 0
    }

    get id() { return this.#id }
    get title() { return this.#title }
    get position() { return this.#position }
    get zIndex() { return this.#zIndex }
    get pos() { return this.#position }
}

class TextElement extends BaseElement {
    #content: string

    constructor()
}