import { Marked } from 'marked'
import { createDirectives } from 'marked-directive'
import DOMPurify from 'dompurify'
import { Color } from './types.ts'


const marked = new Marked()
  .use(createDirectives())

export function parseMarkdown(content: string) {
    // TODO: HTML purify
    const clean = DOMPurify.sanitize(content)
    return marked.parse(clean)
}
export function replaceVariables(content: string, variables: Record<string, string|number|boolean|undefined>) {
    return Object.entries(variables)
      .reduce((acc, [key, value]) => {
        let replacement: string
        if(typeof(value) === "boolean") replacement = value ? 'Yes' : 'No'
        else replacement = String(value)
        return acc.replace(new RegExp(`%${key}%`, 'g'), replacement)
      }, content)
}

export function shouldUseDarkTextParts(r: number, g: number, b: number, a = 1.0) {
  const brightness = r * 0.299 + g * 0.587 + b * 0.114 + (1 - a) * 255;
    
  return brightness > 186
}
export function shouldUseDarkText(color: Color) {
  return shouldUseDarkTextParts(color.r, color.b, color.g, color.a)
}
export function colorToCSS(color?: Color) {
  return color ? `rgba(${color.r}, ${color.g}, ${color.b}, ${color.a ?? 1.0})` : undefined
}