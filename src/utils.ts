export const formatHotkey = (combo: string) => {
  if (!combo) return "";
  return combo.split("+").map(part => {
    if (part === "ControlLeft") return "Left Ctrl";
    if (part === "ControlRight") return "Right Ctrl";
    if (part === "AltLeft") return "Left Alt";
    if (part === "AltRight") return "Right Alt";
    if (part === "ShiftLeft") return "Left Shift";
    if (part === "ShiftRight") return "Right Shift";
    if (part === "MetaLeft") return "Left Win";
    if (part === "MetaRight") return "Right Win";
    if (part === "CommandOrControl" || part === "Control") return "Ctrl";
    if (part === "Alt") return "Alt";
    if (part === "Shift") return "Shift";
    return part;
  }).join(" + ");
};
