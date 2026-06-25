import { describe, expect, it } from 'vitest';
import {
  handleToolbarClickActivation,
  handleToolbarMouseActivation,
  handleToolbarPointerActivation,
  type ToolbarActivationState
} from './toolbarActivation';

function event(button = 0, detail = 1) {
  return {
    button,
    detail,
    preventDefaultCalls: 0,
    preventDefault() {
      this.preventDefaultCalls += 1;
    }
  };
}

describe('toolbar activation sequencing', () => {
  it('runs pointer activation once and suppresses the follow-up mouse and click events', () => {
    const state: ToolbarActivationState = { pointerCommandHandled: false };
    const resets: Array<() => void> = [];
    let commands = 0;

    const pointer = event();
    if (handleToolbarPointerActivation(pointer, state, (callback) => resets.push(callback))) {
      commands += 1;
    }

    const mouse = event();
    if (handleToolbarMouseActivation(mouse, state)) {
      commands += 1;
    }

    const click = event(0, 1);
    if (handleToolbarClickActivation(click)) {
      commands += 1;
    }

    expect(commands).toBe(1);
    expect(pointer.preventDefaultCalls).toBe(1);
    expect(mouse.preventDefaultCalls).toBe(1);
    expect(click.preventDefaultCalls).toBe(1);
    expect(state.pointerCommandHandled).toBe(false);

    resets.forEach((reset) => reset());
    expect(state.pointerCommandHandled).toBe(false);
  });

  it('runs mouse activation once when pointer events are unavailable', () => {
    const state: ToolbarActivationState = { pointerCommandHandled: false };
    let commands = 0;

    const mouse = event();
    if (handleToolbarMouseActivation(mouse, state)) {
      commands += 1;
    }

    const click = event(0, 1);
    if (handleToolbarClickActivation(click)) {
      commands += 1;
    }

    expect(commands).toBe(1);
    expect(mouse.preventDefaultCalls).toBe(1);
    expect(click.preventDefaultCalls).toBe(1);
  });

  it('keeps keyboard click activation available for accessible toolbar use', () => {
    const keyboardClick = event(0, 0);

    expect(handleToolbarClickActivation(keyboardClick)).toBe(true);
    expect(keyboardClick.preventDefaultCalls).toBe(1);
  });

  it('ignores non-primary pointer and mouse activations', () => {
    const state: ToolbarActivationState = { pointerCommandHandled: false };
    const resets: Array<() => void> = [];
    const pointer = event(1);
    const mouse = event(2);

    expect(handleToolbarPointerActivation(pointer, state, (callback) => resets.push(callback))).toBe(false);
    expect(handleToolbarMouseActivation(mouse, state)).toBe(false);
    expect(pointer.preventDefaultCalls).toBe(0);
    expect(mouse.preventDefaultCalls).toBe(0);
    expect(resets).toHaveLength(0);
    expect(state.pointerCommandHandled).toBe(false);
  });
});
