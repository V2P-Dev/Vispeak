export interface DesktopCapabilities {
  native_updater: boolean;
  updater_target: string | null;
  live_typing: boolean;
  wayland: boolean;
  errors: string[];
  record_shortcut: string | null;
  input_ready: boolean;
  caret_available: boolean | null;
}
