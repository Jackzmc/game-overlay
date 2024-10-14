import { Marked } from 'marked'
import { createDirectives } from 'marked-directive'
import DOMPurify from 'dompurify'
import { Color } from './types.ts'
import { invoke } from '@tauri-apps/api'



DOMPurify.addHook( 'afterSanitizeAttributes', function ( node, data, config ) {
  // Set text node content to uppercase
  if ( node.nodeName && node.nodeName.toLowerCase() === 'a' || node.nodeName.toLowerCase() === 'button' ) {
    const href = node.getAttribute('href')
    if(href && !node.hasAttribute('onclick')) {
        node.setAttribute('target', '_blank')
        node.setAttribute('onclick', `return confirm("Are you sure you want to navigate to ${href}?")`)
    } else if ( !node.hasAttribute( 'onclick' ) && node.hasAttribute( "data-action" ) ) {
      const [namespace, command] = node.getAttribute("data-action")!.split(":")
      node.setAttribute( "onclick", `window.InvokeAction("${namespace}", "${command}")`)
    }
  }
} );

async function InvokeAction( namespace: string, command: string ) {
  console.debug( "window.InvokeAction", { namespace, command } )
  await invoke( "perform_action", { namespace, command } )
}
//@ts-expect-error TODO: add to d.ts
window.InvokeAction = InvokeAction

export function sanitize(content: string) {
  return DOMPurify.sanitize(content)
}
export function useTemplate(template: HandlebarsTemplateDelegate, variables: Record<string, any>) {
  return DOMPurify.sanitize(template(variables), {
      SANITIZE_NAMED_PROPS: true,
      ALLOWED_TAGS: ['b', 'pre', 'i', 'em', 'strong', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'p', 'div', 'img', 'a', 'span', 'button', 'buttons'], 
      ALLOWED_ATTR: ['style','class', 'href', 'src', "data-action", "onclick"],
      
      
  })
}

const marked = new Marked()
  .use(createDirectives())

export function parseMarkdown(content: string) {
    const clean = DOMPurify.sanitize(content)
    return marked.parse(clean)
}
export function replaceVariables(content: string, variables: Record<string, any>) {
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