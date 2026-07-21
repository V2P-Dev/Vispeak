import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { enable, disable } from "@tauri-apps/plugin-autostart";
import { Info } from "lucide-react";
import { Language, t, getLanguage } from "../../i18n";
import { applyThemeToDocument } from "../../theme";

interface GeneralPageProps {
  lang: Language;
}

export function GeneralPage({ lang }: GeneralPageProps) {
  const [mics, setMics] = useState<string[]>([]);
  const [selectedMic, setSelectedMic] = useState<string>("default");
  const [gain, setGain] = useState<number>(1.0);
  const [rms, setRms] = useState<number>(0);
  const [autostart, setAutostart] = useState(false);
  const [silentStart, setSilentStart] = useState(false);
  const [soundCues, setSoundCues] = useState(true);
  const [duckAudio, setDuckAudio] = useState(false);
  const [historyLimit, setHistoryLimit] = useState(10);
  const [historySizeMb, setHistorySizeMb] = useState(0);
  const [skin, setSkin] = useState<"full" | "compact" | "mini" | string>("full");
  const [position, setPosition] = useState<string>("bottom-center");
  const [autoUnload, setAutoUnload] = useState<number>(0);

  const [appLanguage, setAppLanguage] = useState<string>("system");
  const [theme, setTheme] = useState<string>("system");
  const [pendingLanguage, setPendingLanguage] = useState<string | null>(null);
  const [hasPendingRestart, setHasPendingRestart] = useState(false);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && pendingLanguage !== null) {
        setPendingLanguage(null);
        setHasPendingRestart(true);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [pendingLanguage]);

  const handleRestartNow = async () => {
    try {
      await invoke("stop_recording");
    } catch(e) {}
    invoke("restart_app");
  };

  useEffect(() => {
    invoke<string[]>("get_microphones").then(setMics);
    invoke<any>("get_settings").then(s => {
      if (s.microphone) setSelectedMic(s.microphone);
      if (s.microphone_gain !== undefined) setGain(s.microphone_gain);
      setAutostart(s.autostart);
      setSilentStart(!!s.silent_start);
      setSoundCues(s.sound_cues !== false);
      if (s.duck_audio) setDuckAudio(!!s.duck_audio);
      if (s.history_limit !== undefined) setHistoryLimit(s.history_limit);
      if (s.overlay_skin) setSkin(s.overlay_skin);
      if (s.overlay_position) setPosition(s.overlay_position);
      if (s.auto_unload_idle_minutes !== undefined) setAutoUnload(s.auto_unload_idle_minutes);
      if (s.app_language) setAppLanguage(s.app_language);
      if (s.theme) setTheme(s.theme); else setTheme("system");
    });
    
    // Start preview when general page opens
    invoke("start_preview");
    
    // Get history size
    invoke<{size_mb: number}>("get_history_size").then(res => setHistorySizeMb(res.size_mb));
    
    return () => {
      // Stop preview when page closes
      invoke("stop_preview");
    };
  }, []);

  useEffect(() => {
    import("@tauri-apps/api/event").then(({ listen }) => {
      const unlisten = listen<number>("audio-level", (event) => {
        setRms(event.payload);
      });
      return () => {
        unlisten.then(f => f());
      };
    });
  }, []);

  const updateSetting = async (key: string, value: any) => {
    console.log(`[settings] Toggle clicked:\nkey: ${key}\nnew value: ${value}`);
    try {
      console.log(`[settings] Sending command:\ncommand: update_single_setting`);
      await invoke("update_single_setting", { key, value });
      console.log(`[settings] Write result: success`);

      // Notify other windows
      await emit("settings-updated");
      
      // Update local state
      if (key === "microphone") setSelectedMic(value || "default");
      if (key === "microphone_gain") setGain(value);
      if (key === "overlay_skin") setSkin(value);
      if (key === "overlay_position") setPosition(value);
      if (key === "theme") {
        setTheme(value);
        applyThemeToDocument(value);
      }
      if (key === "app_language") {
        setAppLanguage(value);
        setPendingLanguage(value);
      }
      if (key === "autostart") {
        console.log("[autostart] Changing setting to:", value);
        if (value) {
          console.log("[autostart] Calling enable()...");
          await enable();
          console.log("[autostart] enable() returned successfully.");
        } else {
          console.log("[autostart] Calling disable()...");
          await disable();
          console.log("[autostart] disable() returned successfully.");
        }
        setAutostart(value);
      }
      if (key === "silent_start") setSilentStart(value);
      if (key === "sound_cues") setSoundCues(value);
      if (key === "duck_audio") setDuckAudio(value);
      if (key === "history_limit") setHistoryLimit(value);
      
      const s = await invoke<any>("get_settings");
      console.log(`[settings] Reload:\nloaded value: ${s[key]}`);
      console.log(`[settings] UI updated`);
    } catch (e) {
      console.error(`[settings] Write result: error -\n`, e);
      if (key === "autostart") {
        setAutostart(!value);
      }
    }
  };

  const handleClearHistory = async () => {
    await invoke("clear_history");
    setHistorySizeMb(0);
  };

  return (
    <div className="w-full max-w-2xl animate-in fade-in slide-in-from-bottom-4 duration-300">
      <div className="mb-8">
        <h2 className="text-3xl font-bold text-primary tracking-tight mb-2">
          {t(lang, "general.title")}
        </h2>
        <p className="text-secondary text-base max-w-2xl">
          {t(lang, "general.subtitle")}
        </p>
      </div>

      <div className="flex flex-col gap-4">
        {/* App Language */}
        <div className="p-5 bg-surface border border-border rounded-2xl flex flex-col gap-3">
          <div className="flex items-center justify-between">
            <label className="text-sm font-medium text-secondary">
              {t(lang, "general.app_language")}
            </label>
            {hasPendingRestart && (
              <span className="text-[11px] font-medium text-secondary/50 bg-border/60 px-2 py-0.5 rounded-full">
                {t(lang, "general.applies_after_restart")}
              </span>
            )}
          </div>
          <div className="relative">
            <select 
              value={appLanguage}
              onChange={(e) => updateSetting("app_language", e.target.value)}
              className="w-full bg-window border border-border rounded-xl px-4 py-3 text-primary appearance-none focus:outline-none focus:ring-1 focus:ring-accent transition-shadow cursor-pointer hover:border-secondary/40"
            >
              <option value="system">{t(lang, "general.app_language_system")}</option>
              <option value="ru">{t(lang, "general.app_language_ru")}</option>
              <option value="en">{t(lang, "general.app_language_en")}</option>
            </select>
            <div className="absolute right-4 top-1/2 -translate-y-1/2 pointer-events-none text-secondary">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <polyline points="6 9 12 15 18 9"></polyline>
              </svg>
            </div>
          </div>
        </div>

        {/* App Theme */}
        <div className="p-5 bg-surface border border-border rounded-2xl flex flex-col gap-4">
          <label className="text-base font-medium text-primary">
            {t(lang, "general.theme")}
          </label>
          <div className="flex gap-4">
            <button
              onClick={() => updateSetting("theme", "system")}
              className={`flex-1 p-4 rounded-xl border-2 flex flex-col items-center gap-3 transition-colors ${theme === "system" ? 'border-accent bg-accent/5' : 'border-border hover:border-border/80'}`}
            >
              <div className="w-full h-16 rounded-lg border border-border/50 flex overflow-hidden relative">
                 <div className="w-1/2 h-full bg-[#0d0d0d] flex items-center justify-center p-2">
                    <div className="w-full h-8 bg-[#1c1c1e] rounded border border-[#2a2a2e]/60 flex items-center justify-center">
                       <div className="w-2 h-2 rounded-full bg-accent/80"></div>
                    </div>
                 </div>
                 <div className="w-1/2 h-full bg-[#f4f4f5] flex items-center justify-center p-2">
                    <div className="w-full h-8 bg-[#ffffff] rounded border border-[#e4e4e7]/80 flex items-center justify-center">
                       <div className="w-2 h-2 rounded-full bg-accent/80"></div>
                    </div>
                 </div>
              </div>
              <span className={`text-sm font-medium ${theme === "system" ? 'text-accent' : 'text-primary'}`}>
                {t(lang, "general.theme_system")}
              </span>
            </button>

            <button
              onClick={() => updateSetting("theme", "dark")}
              className={`flex-1 p-4 rounded-xl border-2 flex flex-col items-center gap-3 transition-colors ${theme === "dark" ? 'border-accent bg-accent/5' : 'border-border hover:border-border/80'}`}
            >
              <div className="w-full h-16 bg-[#0d0d0d] rounded-lg border border-border/50 flex items-center justify-center p-2 relative">
                 <div className="w-full h-10 bg-[#1c1c1e] rounded-md border border-[#2a2a2e] flex items-center px-3 gap-2">
                    <div className="w-2.5 h-2.5 rounded-full bg-[#ff5533]"></div>
                    <div className="flex-1 h-1.5 bg-[#8e8e93]/30 rounded-full"></div>
                 </div>
              </div>
              <span className={`text-sm font-medium ${theme === "dark" ? 'text-accent' : 'text-primary'}`}>
                {t(lang, "general.theme_dark")}
              </span>
            </button>

            <button
              onClick={() => updateSetting("theme", "light")}
              className={`flex-1 p-4 rounded-xl border-2 flex flex-col items-center gap-3 transition-colors ${theme === "light" ? 'border-accent bg-accent/5' : 'border-border hover:border-border/80'}`}
            >
              <div className="w-full h-16 bg-[#f4f4f5] rounded-lg border border-border/50 flex items-center justify-center p-2 relative">
                 <div className="w-full h-10 bg-[#ffffff] rounded-md border border-[#e4e4e7] flex items-center px-3 gap-2 shadow-sm">
                    <div className="w-2.5 h-2.5 rounded-full bg-[#e84a2b]"></div>
                    <div className="flex-1 h-1.5 bg-[#6e6e73]/30 rounded-full"></div>
                 </div>
              </div>
              <span className={`text-sm font-medium ${theme === "light" ? 'text-accent' : 'text-primary'}`}>
                {t(lang, "general.theme_light")}
              </span>
            </button>
          </div>
        </div>

        {/* Microphone */}
        <div className="p-5 bg-surface border border-border rounded-2xl flex flex-col gap-5">
          <div className="flex flex-col gap-3">
            <label className="text-sm font-medium text-secondary">
              {t(lang, "general.microphone")}
            </label>
            <div className="relative">
              <select 
                value={selectedMic}
                onChange={(e) => updateSetting("microphone", e.target.value === "default" ? null : e.target.value)}
                className="w-full appearance-none h-12 px-4 bg-window border border-border rounded-xl text-primary text-sm font-medium hover:border-border/80 focus:border-accent focus:outline-none transition-colors"
              >
                <option value="default">{t(lang, "general.default_mic")}</option>
                {mics.map(m => (
                  <option key={m} value={m}>{m}</option>
                ))}
              </select>
              <div className="absolute right-4 top-1/2 -translate-y-1/2 pointer-events-none text-secondary">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="6 9 12 15 18 9" />
                </svg>
              </div>
            </div>
          </div>
          
          <div className="flex flex-col gap-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2 group">
                <label className="text-sm font-medium text-secondary flex items-center gap-2">
                  {t(lang, "general.gain")}
                  <span className="text-primary font-semibold">{gain.toFixed(1)}x</span>
                </label>
                <div className="relative flex items-center justify-center">
                  <Info className="w-4 h-4 text-secondary opacity-50 cursor-help transition-opacity group-hover:opacity-100" />
                  <div className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2 px-3 py-2 bg-surface border border-border text-primary text-xs rounded-lg w-64 text-center opacity-0 pointer-events-none transition-opacity group-hover:opacity-100 shadow-xl z-10 leading-relaxed">
                    {t(lang, "general.gain_tooltip")}
                  </div>
                </div>
              </div>
              {gain !== 1.0 && (
                <button 
                  onClick={() => updateSetting("microphone_gain", 1.0)}
                  className="text-xs text-accent hover:opacity-80 transition-opacity"
                >
                  {t(lang, "general.reset")}
                </button>
              )}
            </div>
            
            <div className="flex items-center gap-4">
              <input 
                type="range"
                min="0.5"
                max="3.0"
                step="0.1"
                value={gain}
                onChange={(e) => updateSetting("microphone_gain", parseFloat(e.target.value))}
                className="w-full h-1.5 bg-border rounded-full appearance-none [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:bg-accent [&::-webkit-slider-thumb]:rounded-full"
              />
            </div>
            
            <div className="flex items-center gap-2 h-4 w-full">
              <div className="flex-1 flex gap-[2px] h-full items-center">
                {Array.from({ length: 40 }).map((_, i) => {
                  const threshold = (i + 1) / 40;
                  const isActive = (rms * 10) >= threshold;
                  return (
                    <div 
                      key={i} 
                      className={`flex-1 h-full transition-all duration-75 rounded-sm ${isActive ? (i > 32 ? 'bg-red-500' : 'bg-accent') : 'bg-border/50'}`}
                      style={{ opacity: isActive ? 1 : 0.3 }}
                    />
                  );
                })}
              </div>
            </div>
          </div>
        </div>


        {/* Autostart */}
        <div className="p-5 bg-surface border border-border rounded-2xl flex flex-col gap-4">
          <div className="flex items-center justify-between cursor-pointer" onClick={() => updateSetting("autostart", !autostart)}>
            <div className="flex items-center gap-2 group min-w-0">
              <span className="text-base font-medium text-primary truncate">
                {t(lang, "general.autostart")}
              </span>
              <div className="relative flex items-center justify-center">
                <Info className="w-4 h-4 text-secondary opacity-50 cursor-help transition-opacity group-hover:opacity-100" />
                <div className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2 px-3 py-1.5 bg-surface border border-border text-primary text-xs rounded-lg w-48 text-center opacity-0 pointer-events-none transition-opacity group-hover:opacity-100 shadow-xl z-10 whitespace-normal">
                  {t(lang, "general.autostart_desc")}
                </div>
              </div>
            </div>
            <div className={`w-11 h-6 rounded-full p-1 transition-colors ${autostart ? 'bg-accent' : 'bg-border'}`}>
              <div className={`w-4 h-4 bg-white rounded-full transition-transform ${autostart ? 'translate-x-5' : 'translate-x-0'}`} />
            </div>
          </div>
          
          <div className="h-px bg-border w-full"></div>
          
          <div className="flex items-center justify-between cursor-pointer" onClick={() => updateSetting("silent_start", !silentStart)}>
            <div className="flex items-center gap-2 group min-w-0">
              <span className="text-base font-medium text-primary truncate">
                {t(lang, "general.silent_start")}
              </span>
              <div className="relative flex items-center justify-center">
                <Info className="w-4 h-4 text-secondary opacity-50 cursor-help transition-opacity group-hover:opacity-100" />
                <div className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2 px-3 py-1.5 bg-surface border border-border text-primary text-xs rounded-lg w-48 text-center opacity-0 pointer-events-none transition-opacity group-hover:opacity-100 shadow-xl z-10 whitespace-normal">
                  {t(lang, "general.silent_start_desc")}
                </div>
              </div>
            </div>
            <div className={`w-11 h-6 rounded-full p-1 transition-colors ${silentStart ? 'bg-accent' : 'bg-border'}`}>
              <div className={`w-4 h-4 bg-white rounded-full transition-transform ${silentStart ? 'translate-x-5' : 'translate-x-0'}`} />
            </div>
          </div>
        </div>
        
        {/* Audio behavior */}
        <div className="p-5 bg-surface border border-border rounded-2xl flex flex-col gap-4">
          <div className="flex items-center justify-between cursor-pointer" onClick={() => updateSetting("sound_cues", !soundCues)}>
            <div className="flex items-center gap-2 group min-w-0">
              <span className="text-base font-medium text-primary truncate">
                {t(lang, "general.sound_cues")}
              </span>
              <div className="relative flex items-center justify-center">
                <Info className="w-4 h-4 text-secondary opacity-50 cursor-help transition-opacity group-hover:opacity-100" />
                <div className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2 px-3 py-1.5 bg-surface border border-border text-primary text-xs rounded-lg w-48 text-center opacity-0 pointer-events-none transition-opacity group-hover:opacity-100 shadow-xl z-10 whitespace-normal">
                  {t(lang, "general.sound_cues_desc")}
                </div>
              </div>
            </div>
            <div className={`w-11 h-6 rounded-full p-1 transition-colors ${soundCues ? 'bg-accent' : 'bg-border'}`}>
              <div className={`w-4 h-4 bg-white rounded-full transition-transform ${soundCues ? 'translate-x-5' : 'translate-x-0'}`} />
            </div>
          </div>
          
          <div className="h-px bg-border w-full"></div>
          
          <div className="flex items-center justify-between cursor-pointer" onClick={() => updateSetting("duck_audio", !duckAudio)}>
            <div className="flex items-center gap-2 group min-w-0">
              <span className="text-base font-medium text-primary truncate">
                {t(lang, "general.duck_audio")}
              </span>
              <div className="relative flex items-center justify-center">
                <Info className="w-4 h-4 text-secondary opacity-50 cursor-help transition-opacity group-hover:opacity-100" />
                <div className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2 px-3 py-1.5 bg-surface border border-border text-primary text-xs rounded-lg w-48 text-center opacity-0 pointer-events-none transition-opacity group-hover:opacity-100 shadow-xl z-10 whitespace-normal">
                  {t(lang, "general.duck_audio_desc")}
                </div>
              </div>
            </div>
            <div className={`w-11 h-6 rounded-full p-1 transition-colors ${duckAudio ? 'bg-accent' : 'bg-border'}`}>
              <div className={`w-4 h-4 bg-white rounded-full transition-transform ${duckAudio ? 'translate-x-5' : 'translate-x-0'}`} />
            </div>
          </div>
        </div>

        {/* Free memory when idle */}
        <div className="p-5 bg-surface border border-border rounded-2xl flex flex-col gap-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 group min-w-0">
              <span className="text-base font-medium text-primary truncate">
                {t(lang, "general.auto_unload")}
              </span>
              <div className="relative flex items-center justify-center">
                <Info className="w-4 h-4 text-secondary opacity-50 cursor-help transition-opacity group-hover:opacity-100" />
                <div className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2 px-3 py-1.5 bg-surface border border-border text-primary text-xs rounded-lg w-64 text-center opacity-0 pointer-events-none transition-opacity group-hover:opacity-100 shadow-xl z-10 whitespace-normal">
                  {t(lang, "general.auto_unload_desc")}
                </div>
              </div>
            </div>
            <div className="relative w-48 shrink-0">
              <select 
                value={autoUnload}
                onChange={(e) => {
                  const val = parseInt(e.target.value);
                  setAutoUnload(val);
                  updateSetting("auto_unload_idle_minutes", val);
                }}
                className="w-full appearance-none h-10 px-4 bg-window border border-border rounded-xl text-primary text-sm font-medium hover:border-border/80 focus:border-accent focus:outline-none transition-colors"
              >
                <option value={0}>{t(lang, "general.auto_unload_never")}</option>
                <option value={1}>{t(lang, "general.auto_unload_1min")}</option>
                <option value={5}>{t(lang, "general.auto_unload_5min")}</option>
                <option value={15}>{t(lang, "general.auto_unload_15min")}</option>
              </select>
              <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-secondary">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="6 9 12 15 18 9" />
                </svg>
              </div>
            </div>
          </div>
        </div>

        {/* History */}
        <div className="p-5 bg-surface border border-border rounded-2xl flex flex-col gap-4">
          <div className="flex items-center justify-between">
            <label className="text-base font-medium text-primary">
              {t(lang, "general.history_limit")}
            </label>
            <div className="relative w-48">
              <select 
                value={historyLimit}
                onChange={(e) => updateSetting("history_limit", parseInt(e.target.value))}
                className="w-full appearance-none h-10 px-4 bg-window border border-border rounded-xl text-primary text-sm font-medium hover:border-border/80 focus:border-accent focus:outline-none transition-colors"
              >
                <option value={0}>{t(lang, "general.history_limit_0")}</option>
                <option value={5}>{t(lang, "general.history_limit_5")}</option>
                <option value={10}>{t(lang, "general.history_limit_10")}</option>
              </select>
              <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-secondary">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="6 9 12 15 18 9" />
                </svg>
              </div>
            </div>
          </div>
          
          <div className="h-px bg-border w-full"></div>
          
          <div className="flex items-center justify-between">
            <span className="text-sm text-secondary">
              {t(lang, "general.history_size", { size: historySizeMb.toFixed(1) })}
            </span>
            <button
              onClick={handleClearHistory}
              className="text-sm font-medium text-[#ff453a] hover:bg-[#ff453a]/10 px-3 py-1.5 rounded-lg transition-colors border border-transparent hover:border-[#ff453a]/20"
            >
              {t(lang, "general.history_clear")}
            </button>
          </div>
        </div>
        
        {/* Overlay Position */}
        <div className="p-5 bg-surface border border-border rounded-2xl flex flex-col gap-3">
          <label className="text-base font-medium text-primary flex items-center gap-2">
            <span>{t(lang, "general.overlay_position")}</span>
            {skin === "mini" && (
              <span className="text-xs text-secondary font-normal">{t(lang, "general.pos_fallback_hint")}</span>
            )}
          </label>
          <div className="relative">
            <select 
              value={position}
              onChange={(e) => updateSetting("overlay_position", e.target.value)}
              className="w-full appearance-none h-12 px-4 bg-window border border-border rounded-xl text-primary text-sm font-medium hover:border-border/80 focus:border-accent focus:outline-none transition-colors"
            >
              <option value="bottom-center">{t(lang, "general.pos_bottom_center")}</option>
              <option value="bottom-right">{t(lang, "general.pos_bottom_right")}</option>
              <option value="bottom-left">{t(lang, "general.pos_bottom_left")}</option>
              <option value="top-center">{t(lang, "general.pos_top_center")}</option>
            </select>
            <div className="absolute right-4 top-1/2 -translate-y-1/2 pointer-events-none text-secondary">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <polyline points="6 9 12 15 18 9" />
              </svg>
            </div>
          </div>
        </div>

        {/* Overlay Skin */}
        <div className="p-5 bg-surface border border-border rounded-2xl flex flex-col gap-4">
          <label className="text-base font-medium text-primary">
            {t(lang, "general.overlay_skin")}
          </label>
          <div className="flex gap-4">
            <button
              onClick={() => updateSetting("overlay_skin", "full")}
              className={`flex-1 p-4 rounded-xl border-2 flex flex-col items-center gap-3 transition-colors ${skin === "full" ? 'border-accent bg-accent/5' : 'border-border hover:border-border/80'}`}
            >
              <div className="w-full h-16 bg-overlay rounded-lg border border-border/50 flex flex-col p-2 gap-1.5 relative overflow-hidden">
                 <div className="flex justify-between items-center w-full">
                    <div className="w-12 h-2 bg-secondary/30 rounded-full"></div>
                    <div className="w-8 h-2 bg-secondary/20 rounded-full"></div>
                 </div>
                 <div className="flex-1 w-full bg-accent/20 rounded-md flex items-center justify-center">
                    <div className="w-1/2 h-1 bg-accent/50 rounded-full"></div>
                 </div>
              </div>
              <span className={`text-sm font-medium ${skin === "full" ? 'text-accent' : 'text-primary'}`}>
                {t(lang, "general.skin_full")}
              </span>
            </button>

            <button
              onClick={() => updateSetting("overlay_skin", "compact")}
              className={`flex-1 p-4 rounded-xl border-2 flex flex-col items-center gap-3 transition-colors ${skin === "compact" ? 'border-accent bg-accent/5' : 'border-border hover:border-border/80'}`}
            >
              <div className="w-full h-16 bg-window rounded-lg flex flex-col items-center justify-center gap-1.5 relative border border-border/40">
                 <div className="w-[60%] h-6 bg-overlay rounded-full border border-border/50 flex items-center px-2 gap-2">
                    <div className="w-2 h-2 rounded-full bg-accent/80"></div>
                    <div className="flex-1 h-1 bg-accent/30 rounded-full"></div>
                 </div>
              </div>
              <span className={`text-sm font-medium ${skin === "compact" ? 'text-accent' : 'text-primary'}`}>
                {t(lang, "general.skin_compact")}
              </span>
            </button>

            <button
              onClick={() => updateSetting("overlay_skin", "mini")}
              className={`flex-1 p-4 rounded-xl border-2 flex flex-col items-center gap-3 transition-colors ${skin === "mini" ? 'border-accent bg-accent/5' : 'border-border hover:border-border/80'}`}
            >
              <div className="w-full h-16 bg-window rounded-lg flex flex-col items-center justify-center gap-1.5 relative border border-border/40">
                 <div className="w-[32%] h-5 bg-overlay rounded-full border border-border/50 flex items-center justify-center px-1">
                    <div className="w-full h-1 bg-accent/60 rounded-full"></div>
                 </div>
              </div>
              <span className={`text-sm font-medium ${skin === "mini" ? 'text-accent' : 'text-primary'}`}>
                {t(lang, "general.skin_mini")}
              </span>
            </button>
          </div>
          {skin === "mini" && (
            <div className="text-xs text-secondary bg-window p-3 rounded-xl border border-border/50 leading-relaxed">
              {t(lang, "general.skin_mini_desc")}
            </div>
          )}
        </div>
      </div>

      {/* Restart Modal */}
      {pendingLanguage !== null && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <div 
            className="absolute inset-0 bg-black/60 dark:bg-black/60 data-[theme=light]:bg-black/30 backdrop-blur-sm"
            onClick={() => {
              setPendingLanguage(null);
              setHasPendingRestart(true);
            }}
          />
          <div className="relative bg-surface border border-border rounded-2xl p-6 shadow-2xl max-w-sm w-full mx-4 animate-in zoom-in-95 duration-200 flex flex-col gap-4">
            <h3 className="text-xl font-bold text-primary">
              {t(getLanguage(pendingLanguage), "general.restart_required")}
            </h3>
            <p className="text-secondary text-sm">
              {t(getLanguage(pendingLanguage), "general.restart_message")}
            </p>
            <div className="flex gap-3 mt-2">
              <button
                className="flex-1 bg-accent text-accent-text font-semibold py-2.5 rounded-xl hover:bg-accent/90 transition-colors"
                onClick={handleRestartNow}
              >
                {t(getLanguage(pendingLanguage), "general.restart_now")}
              </button>
              <button
                className="flex-1 bg-window border border-border text-primary font-medium py-2.5 rounded-xl hover:border-secondary/40 transition-colors"
                onClick={() => {
                  setPendingLanguage(null);
                  setHasPendingRestart(true);
                }}
              >
                {t(getLanguage(pendingLanguage), "general.restart_later")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
