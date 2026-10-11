/**
 * Svelte action: adds a Material-style ripple that expands from the click
 * point (or the element's center, for keyboard activation) and fades out.
 */
export function ripple (node: HTMLElement): { destroy: () => void } {
  node.classList.add('ripple-container')

  let current: HTMLSpanElement | null = null

  function handleClick (event: MouseEvent): void {
    // A new click cancels whatever ripple is still animating, so the
    // button never looks like it's waiting on the previous one.
    current?.remove()

    const rect = node.getBoundingClientRect()
    const fromCenter = event.detail === 0
    const x = fromCenter ? rect.width / 2 : event.clientX - rect.left
    const y = fromCenter ? rect.height / 2 : event.clientY - rect.top

    const maxX = Math.max(x, rect.width - x)
    const maxY = Math.max(y, rect.height - y)
    const radius = Math.sqrt(maxX * maxX + maxY * maxY)
    const diameter = radius * 2

    const span = document.createElement('span')
    span.className = 'ripple-effect'
    span.style.width = `${diameter}px`
    span.style.height = `${diameter}px`
    span.style.left = `${x - radius}px`
    span.style.top = `${y - radius}px`

    node.appendChild(span)
    current = span

    span.addEventListener('animationend', () => {
      if (current === span) current = null
      span.remove()
    }, { once: true })
  }

  node.addEventListener('click', handleClick)

  return {
    destroy () {
      node.removeEventListener('click', handleClick)
      current?.remove()
    }
  }
}
