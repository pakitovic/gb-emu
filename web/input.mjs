const KEY_TO_PLAYER_BUTTON = {
  ArrowRight: { player: 0, button: 0 },
  ArrowLeft: { player: 0, button: 1 },
  ArrowUp: { player: 0, button: 2 },
  ArrowDown: { player: 0, button: 3 },
  KeyZ: { player: 0, button: 5 },
  KeyX: { player: 0, button: 4 },
  Backspace: { player: 0, button: 6 },
  Enter: { player: 0, button: 7 },
  KeyD: { player: 1, button: 0 },
  KeyA: { player: 1, button: 1 },
  KeyW: { player: 1, button: 2 },
  KeyS: { player: 1, button: 3 },
  KeyF: { player: 1, button: 5 },
  KeyG: { player: 1, button: 4 },
  KeyR: { player: 1, button: 6 },
  KeyT: { player: 1, button: 7 },
  KeyL: { player: 2, button: 0 },
  KeyJ: { player: 2, button: 1 },
  KeyI: { player: 2, button: 2 },
  KeyK: { player: 2, button: 3 },
  KeyU: { player: 2, button: 5 },
  KeyO: { player: 2, button: 4 },
  KeyY: { player: 2, button: 6 },
  KeyP: { player: 2, button: 7 },
  Numpad6: { player: 3, button: 0 },
  Numpad4: { player: 3, button: 1 },
  Numpad8: { player: 3, button: 2 },
  Numpad5: { player: 3, button: 3 },
  Numpad1: { player: 3, button: 5 },
  Numpad2: { player: 3, button: 4 },
  Numpad7: { player: 3, button: 6 },
  Numpad9: { player: 3, button: 7 },
};

export function bindKeyboardInput({ target = window, getEmulator }) {
  function setPlayerButton(emulator, playerIndex, buttonIndex, pressed) {
    if (typeof emulator.set_player_button === "function") {
      emulator.set_player_button(playerIndex, buttonIndex, pressed);
      return;
    }
    if (playerIndex === 0 && typeof emulator.set_button === "function") {
      emulator.set_button(buttonIndex, pressed);
    }
  }

  function setButtonFromKeyboardEvent(event, pressed) {
    const emulator = getEmulator?.();
    if (!emulator) {
      return;
    }
    const mapped = KEY_TO_PLAYER_BUTTON[event.code];
    if (!mapped) {
      return;
    }
    event.preventDefault();
    setPlayerButton(emulator, mapped.player, mapped.button, pressed);
  }

  const onKeyDown = (event) => setButtonFromKeyboardEvent(event, true);
  const onKeyUp = (event) => setButtonFromKeyboardEvent(event, false);

  target.addEventListener("keydown", onKeyDown);
  target.addEventListener("keyup", onKeyUp);

  return () => {
    target.removeEventListener("keydown", onKeyDown);
    target.removeEventListener("keyup", onKeyUp);
  };
}
