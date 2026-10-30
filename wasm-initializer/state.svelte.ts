export const loadingState = $state({
  status: "idle" as "idle" | "loading" | "ready" | "error",
  current: 0,
  total: 0,
  error: null as unknown,
  visible: true,
})

export function start() {
  loadingState.status = "loading"
  loadingState.current = 0
  loadingState.total = 0
  loadingState.error = null
  loadingState.visible = true
}

export function progress(current: number, total: number) {
  if (loadingState.status !== "loading") return

  loadingState.current = current
  loadingState.total = total
}

export function done() {
  loadingState.status = "ready"
  loadingState.current = loadingState.total || loadingState.current
}

export function fail(error: unknown) {
  loadingState.status = "error"
  loadingState.error = error
}

export function finishFirstScreen() {
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      loadingState.visible = false
    })
  })
}
