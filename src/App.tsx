import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { MainWindow } from "./components/MainWindow";
import { OverlayWindow } from "./components/OverlayWindow";
import { UpdaterProvider } from "./components/UpdaterProvider";
import { applyThemeToDocument } from "./theme";
import "./App.css";

function App() {
  const [windowLabel, setWindowLabel] = useState<string | null>(null);
  const [_themeSetting, setThemeSetting] = useState<string>("system");

  useEffect(() => {
    const appWindow = getCurrentWindow();
    setWindowLabel(appWindow.label);

    if (appWindow.label === "overlay") {
      // INTENTIONAL: Overlay indicator stays ALWAYS DARK regardless of the theme choice.
      document.documentElement.setAttribute("data-theme", "dark");
      return;
    }

    // For MainWindow, fetch initial theme setting from Rust backend
    invoke<any>("get_settings").then((settings) => {
      const t = settings.theme || "system";
      setThemeSetting(t);
      applyThemeToDocument(t);
    }).catch(console.error);

    const unlistenSettings = listen("settings-updated", () => {
      invoke<any>("get_settings").then((settings) => {
        const t = settings.theme || "system";
        setThemeSetting(t);
        applyThemeToDocument(t);
      }).catch(console.error);
    });

    // Listen for system theme changes (prefers-color-scheme)
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handleSystemThemeChange = () => {
      invoke<any>("get_settings").then((settings) => {
        const t = settings.theme || "system";
        if (t === "system") {
          applyThemeToDocument("system");
        }
      }).catch(console.error);
    };
    mediaQuery.addEventListener("change", handleSystemThemeChange);

    return () => {
      unlistenSettings.then(f => f());
      mediaQuery.removeEventListener("change", handleSystemThemeChange);
    };
  }, []);

  if (!windowLabel) return null;

  if (windowLabel === "overlay") {
    // INTENTIONAL: Overlay indicator stays ALWAYS DARK regardless of the theme choice.
    return (
      <div data-theme="dark" className="w-full h-full">
        <OverlayWindow />
      </div>
    );
  }

  return (
    <div className="w-full h-full bg-window overflow-hidden text-primary transition-colors duration-150">
      <UpdaterProvider>
        <MainWindow />
      </UpdaterProvider>
    </div>
  );
}

export default App;
