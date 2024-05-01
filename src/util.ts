import { Marked } from 'marked'
import { createDirectives } from 'marked-directive'
import DOMPurify from 'dompurify'


const marked = new Marked()
  .use(createDirectives())

export function parseMarkdown(content: string) {
    // TODO: HTML purify
    const clean = DOMPurify.sanitize(content)
    return marked.parse(clean)
}
