import { Marked } from 'marked'
import { createDirectives } from 'marked-directive'


const marked = new Marked()
  .use(createDirectives())

export function parseMarkdown(content: string) {
    // TODO: HTML purify
    return marked.parse(content)
}