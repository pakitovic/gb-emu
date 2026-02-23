const KEY_TO_BUTTON = {
  ArrowRight: 0,
  ArrowLeft: 1,
  ArrowUp: 2,
  ArrowDown: 3,
  KeyZ: 4,
  KeyX: 5,
  Backspace: 6,
  Enter: 7,
};

export function bindKeyboardInput({ target = window, getEmulator }) {
  function setButtonFromKeyboardEvent(event, pressed) {
    const emulator = getEmulator?.();
    if (!emulator) {
      return;
    }
    const buttonIndex = KEY_TO_BUTTON[event.code];
    if (buttonIndex === undefined) {
      return;
    }
    event.preventDefault();
    emulator.set_button(buttonIndex, pressed);
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
