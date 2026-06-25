export interface ToolbarActivationState {
  pointerCommandHandled: boolean;
}

export interface ToolbarMouseLikeEvent {
  button: number;
  preventDefault(): void;
}

export interface ToolbarClickLikeEvent {
  detail: number;
  preventDefault(): void;
}

export type ToolbarResetScheduler = (callback: () => void) => unknown;

export function handleToolbarPointerActivation(
  event: ToolbarMouseLikeEvent,
  state: ToolbarActivationState,
  scheduleReset: ToolbarResetScheduler
): boolean {
  if (event.button !== 0) {
    return false;
  }
  state.pointerCommandHandled = true;
  scheduleReset(() => {
    state.pointerCommandHandled = false;
  });
  event.preventDefault();
  return true;
}

export function handleToolbarMouseActivation(
  event: ToolbarMouseLikeEvent,
  state: ToolbarActivationState
): boolean {
  if (event.button !== 0) {
    return false;
  }
  if (state.pointerCommandHandled) {
    state.pointerCommandHandled = false;
    event.preventDefault();
    return false;
  }
  event.preventDefault();
  return true;
}

export function handleToolbarClickActivation(event: ToolbarClickLikeEvent): boolean {
  event.preventDefault();
  return event.detail === 0;
}
