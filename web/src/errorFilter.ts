const CONTROL_FLOW_ERROR = "Using exceptions for control flow";

function isControlFlowError(value: unknown): boolean {
  return (
    (value instanceof Error && value.message.includes(CONTROL_FLOW_ERROR)) ||
    (typeof value === "string" && value.includes(CONTROL_FLOW_ERROR))
  );
}

window.addEventListener("error", (event) => {
  if (
    isControlFlowError(event.error) ||
    event.message.includes(CONTROL_FLOW_ERROR)
  ) {
    event.preventDefault();
  }
});

window.addEventListener("unhandledrejection", (event) => {
  if (isControlFlowError(event.reason)) {
    event.preventDefault();
  }
});
