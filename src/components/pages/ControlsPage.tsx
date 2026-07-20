import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Info, RotateCcw } from "lucide-react";
import { Language, t } from "../../i18n";
import { formatHotkey } from "../../utils";
import { ModelInfo } from "./ModelPage";

interface ControlsPageProps {
  lang: Language;
}

export function ControlsPage({ lang }: ControlsPageProps) {
  const [currentHotkey, setCurrentHotkey] = useState("");
  const [inputHotkey, setInputHotkey] = useState("");
  const [isRecordingHotkey, setIsRecordingHotkey] = useState(false);

  const [currentCancelHotkey, setCurrentCancelHotkey] = useState("");
  const [inputCancelHotkey, setInputCancelHotkey] = useState("");
  const [isRecordingCancelHotkey, setIsRecordingCancelHotkey] = useState(false);

  const [pushToTalk, setPushToTalk] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [models, setModels] = useState<ModelInfo[]>([]);
  const [modelSettings, setModelSettings] = useState<Record<string, { language: string, initial_prompt?: string | null }>>({});
  const [activeModelId, setActiveModelId] = useState<string | null>(null);
  
  const [textInputMethod, setTextInputMethod] = useState("paste");
  const [clipboardAfter, setClipboardAfter] = useState("restore");
  const [trailingSpace, setTrailingSpace] = useState(false);
  const [sendAfter, setSendAfter] = useState("none");

  useEffect(() => {
    invoke<any>("get_settings").then(s => {
      setCurrentHotkey(s.hotkey);
      setInputHotkey(s.hotkey);
      setCurrentCancelHotkey(s.cancel_hotkey || "Escape");
      setInputCancelHotkey(s.cancel_hotkey || "Escape");
      setPushToTalk(!!s.push_to_talk);
      setModelSettings(s.model_settings || {});
      setActiveModelId(s.active_model || null);
      setTextInputMethod(s.text_input_method || "paste");
      setClipboardAfter(s.clipboard_after || "restore");
      setTrailingSpace(!!s.trailing_space);
      setSendAfter(s.send_after || "none");
    });
    invoke<ModelInfo[]>("get_models").then(setModels);
  }, []);

  const saveSettings = async (updates: { hotkey?: string; cancel_hotkey?: string; push_to_talk?: boolean }) => {
    setError(null);
    try {
      if (updates.hotkey !== undefined) {
        if (areHotkeysConflict(updates.hotkey, currentCancelHotkey)) throw new Error("conflict");
        await invoke("update_hotkey", { newKey: updates.hotkey });
        setCurrentHotkey(updates.hotkey);
      }
      if (updates.cancel_hotkey !== undefined) {
        if (areHotkeysConflict(updates.cancel_hotkey, currentHotkey)) throw new Error("conflict");
        await invoke("update_cancel_hotkey", { newKey: updates.cancel_hotkey });
        setCurrentCancelHotkey(updates.cancel_hotkey);
      }
      if (updates.push_to_talk !== undefined) {
        await invoke("update_push_to_talk", { enabled: updates.push_to_talk });
        setPushToTalk(updates.push_to_talk);
      }
    } catch (err: any) {
      setError(err.toString());
      if (updates.hotkey) setInputHotkey(currentHotkey);
      if (updates.cancel_hotkey) setInputCancelHotkey(currentCancelHotkey);
    }
  };

  const areHotkeysConflict = (h1: string, h2: string) => {
    if (!h1 || !h2) return false;
    const s1 = h1.split("+").sort().join("+");
    const s2 = h2.split("+").sort().join("+");
    return s1 === s2;
  };

  const mapCodeToKeyName = (code: string, key: string) => {
    if (code.startsWith("Key")) return code.replace("Key", "");
    if (code.startsWith("Digit")) return code.replace("Digit", "");
    if (code.startsWith("Arrow")) return code.replace("Arrow", "");
    if (code === "Space") return "Space";
    if (code === "Enter") return "Enter";
    if (code === "Escape") return "Escape";
    if (code === "Backspace") return "Backspace";
    if (code === "Tab") return "Tab";
    if (code.startsWith("F") && code.length <= 3) return code;
    if (["Insert", "Delete", "Home", "End", "PageUp", "PageDown"].includes(code)) return code;
    if (["ControlLeft", "ControlRight", "ShiftLeft", "ShiftRight", "AltLeft", "AltRight", "MetaLeft", "MetaRight"].includes(code)) return code;
    if (code === "AltGraph") return "AltRight";
    
    if (key === " ") return "Space";
    if (key.length === 1) return key.toUpperCase();
    return code;
  };

  const handleKeyDown = (e: React.KeyboardEvent, isCancel: boolean) => {
    e.preventDefault();
    setError(null);
    
    if (isCancel && !isRecordingCancelHotkey) return;
    if (!isCancel && !isRecordingHotkey) return;
    
    const keyStr = mapCodeToKeyName(e.code, e.key);
    
    // We only need the latest valid combo string. We don't need a React state Set because we can just accumulate strings.
    // Actually, `e.ctrlKey`, `e.shiftKey`, `e.altKey` are enough to build the base if we want, but since we map `e.code`, we just overwrite the string for simplicity.
    // Wait, the previous logic accumulated them. Let's just do it directly.
    const parts = [];
    if (e.ctrlKey || e.metaKey) parts.push("Control");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");
    
    // Add the main key if it's not a modifier
    if (!["ControlLeft", "ControlRight", "ShiftLeft", "ShiftRight", "AltLeft", "AltRight", "MetaLeft", "MetaRight", "Control", "Shift", "Alt", "Meta"].includes(keyStr)) {
        parts.push(keyStr);
    } else {
        // If it's a standalone modifier and nothing else is pressed, we can use it as the main key
        if (parts.length === 0 || (parts.length === 1 && parts[0] === keyStr.replace(/Left|Right/, ""))) {
            parts.length = 0; // Clear
            parts.push(keyStr);
        }
    }
    
    const combo = parts.join("+");
    if (isCancel) setInputCancelHotkey(combo);
    else setInputHotkey(combo);
  };

  const handleKeyUp = (e: React.KeyboardEvent, isCancel: boolean) => {
    e.preventDefault();
    if (isCancel && !isRecordingCancelHotkey) return;
    if (!isCancel && !isRecordingHotkey) return;
    
    // Once a key is released, we consider the combination complete
    if (isCancel) {
      setIsRecordingCancelHotkey(false);
      saveSettings({ cancel_hotkey: inputCancelHotkey });
    } else {
      setIsRecordingHotkey(false);
      saveSettings({ hotkey: inputHotkey });
    }
  };

  const handleResetHotkey = () => {
    setInputHotkey("Control+Space");
    saveSettings({ hotkey: "Control+Space" });
  };
  const handleResetCancelHotkey = () => {
    setInputCancelHotkey("Escape");
    saveSettings({ cancel_hotkey: "Escape" });
  };

  const saveModelSetting = async (modelId: string, key: string, value: string | null) => {
    const newModelSettings = {
      ...modelSettings,
      [modelId]: {
        ...(modelSettings[modelId] || { language: "auto" }),
        [key]: value
      }
    };
    await invoke("update_single_setting", { key: "model_settings", value: newModelSettings });
    setModelSettings(newModelSettings);
  };

  const updateTextInputSetting = async (key: string, value: any) => {
    await invoke("update_single_setting", { key, value });
    if (key === "text_input_method") setTextInputMethod(value);
    if (key === "clipboard_after") setClipboardAfter(value);
    if (key === "trailing_space") setTrailingSpace(value);
    if (key === "send_after") setSendAfter(value);
  };

  const activeModel = models.find(m => m.id === activeModelId);
  const activeSettings = activeModelId ? (modelSettings[activeModelId] || { language: "auto", initial_prompt: "" }) : null;
  const isWhisper = activeModelId ? ["tiny", "base", "small"].includes(activeModelId) : false;

  const renderLanguageOptions = () => {
    if (activeModel?.id === "canary") {
      return [
        { value: "en", label: t(lang, "controls.lang_opts.en") },
        { value: "de", label: t(lang, "controls.lang_opts.de") },
        { value: "fr", label: t(lang, "controls.lang_opts.fr") },
        { value: "es", label: t(lang, "controls.lang_opts.es") }
      ];
    }
    return [
      { value: "auto", label: t(lang, "controls.lang_opts.auto") },
      { value: "ru", label: t(lang, "controls.lang_opts.ru") },
      { value: "en", label: t(lang, "controls.lang_opts.en") },
      { value: "de", label: t(lang, "controls.lang_opts.de") },
      { value: "fr", label: t(lang, "controls.lang_opts.fr") },
      { value: "es", label: t(lang, "controls.lang_opts.es") }
    ];
  };

  return (
    <div className="w-full max-w-2xl animate-in fade-in slide-in-from-bottom-4 duration-300">
      <div className="mb-8">
        <h2 className="text-3xl font-bold text-primary tracking-tight mb-2">
          {t(lang, "controls.title")}
        </h2>
        <p className="text-secondary text-base max-w-2xl">
          {t(lang, "controls.subtitle")}
        </p>
      </div>

      <div className="bg-surface border border-border rounded-2xl flex flex-col">
        
        {/* Record Hotkey Row */}
        <div className="flex flex-wrap items-center justify-between gap-4 p-5 border-b border-border">
          <div className="flex items-center gap-2 group min-w-0">
            <span className="text-sm font-medium text-primary truncate">{t(lang, "controls.record_hotkey")}</span>
            <div className="relative flex items-center justify-center">
              <Info className="w-4 h-4 text-secondary opacity-50 cursor-help transition-opacity group-hover:opacity-100" />
              <div className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2 px-3 py-1.5 bg-surface border border-border text-primary text-xs rounded-lg whitespace-nowrap opacity-0 pointer-events-none transition-opacity group-hover:opacity-100 shadow-xl z-10">
                {t(lang, "controls.record_hotkey_tooltip")}
              </div>
            </div>
          </div>
          <div className="flex items-center gap-3">
            <div 
              className={`h-9 px-4 rounded-xl flex items-center justify-center cursor-pointer transition-all outline-none select-none ${
                isRecordingHotkey 
                  ? 'border border-accent shadow-[var(--shadow-accent-md)] bg-window' 
                  : 'border border-border bg-window hover:border-border/80'
              }`}
              tabIndex={0}
              onFocus={() => { setIsRecordingHotkey(true); setInputHotkey(""); }}
              onBlur={() => { setIsRecordingHotkey(false); setInputHotkey(currentHotkey); }}
              onKeyDown={(e) => handleKeyDown(e, false)}
              onKeyUp={(e) => handleKeyUp(e, false)}
            >
              <span className={`text-sm font-medium tracking-wide ${isRecordingHotkey ? 'text-accent' : 'text-primary'}`}>
                {isRecordingHotkey ? (inputHotkey ? formatHotkey(inputHotkey) : "...") : formatHotkey(currentHotkey)}
              </span>
            </div>
            <button 
              onClick={handleResetHotkey}
              className="text-secondary opacity-50 hover:opacity-100 transition-opacity"
              title="Reset to default (Ctrl+Space)"
            >
              <RotateCcw className="w-4 h-4" />
            </button>
          </div>
        </div>

        {/* Push-to-Talk Row */}
        <div className="flex flex-wrap items-center justify-between gap-4 p-5 border-b border-border">
          <div className="flex items-center gap-2 group min-w-0">
            <span className="text-sm font-medium text-primary truncate">{t(lang, "controls.ptt")}</span>
            <div className="relative flex items-center justify-center">
              <Info className="w-4 h-4 text-secondary opacity-50 cursor-help transition-opacity group-hover:opacity-100" />
              <div className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2 px-3 py-1.5 bg-surface border border-border text-primary text-xs rounded-lg whitespace-nowrap opacity-0 pointer-events-none transition-opacity group-hover:opacity-100 shadow-xl z-10">
                {t(lang, "controls.ptt_tooltip")}
              </div>
            </div>
          </div>
          
          <button
            onClick={() => saveSettings({ push_to_talk: !pushToTalk })}
            className={`w-11 h-6 rounded-full transition-colors relative ${pushToTalk ? 'bg-accent' : 'bg-window border border-border'}`}
          >
            <div className={`w-5 h-5 bg-knob rounded-full absolute top-0.5 shadow-sm transition-all duration-300 ${pushToTalk ? 'left-[22px]' : 'left-[3px] opacity-70'}`}></div>
          </button>
        </div>

        {/* Cancel Hotkey Row */}
        <div className="flex flex-wrap items-center justify-between gap-4 p-5">
          <div className="flex items-center gap-2 group min-w-0">
            <span className="text-sm font-medium text-primary truncate">{t(lang, "controls.cancel_hotkey")}</span>
            <div className="relative flex items-center justify-center">
              <Info className="w-4 h-4 text-secondary opacity-50 cursor-help transition-opacity group-hover:opacity-100" />
              <div className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2 px-3 py-1.5 bg-surface border border-border text-primary text-xs rounded-lg whitespace-nowrap opacity-0 pointer-events-none transition-opacity group-hover:opacity-100 shadow-xl z-10">
                {t(lang, "controls.cancel_hotkey_tooltip")}
              </div>
            </div>
          </div>
          <div className="flex items-center gap-3">
            <div 
              className={`h-9 px-4 rounded-xl flex items-center justify-center cursor-pointer transition-all outline-none select-none ${
                isRecordingCancelHotkey 
                  ? 'border border-accent shadow-[var(--shadow-accent-md)] bg-window' 
                  : 'border border-border bg-window hover:border-border/80'
              }`}
              tabIndex={0}
              onFocus={() => { setIsRecordingCancelHotkey(true); setInputCancelHotkey(""); }}
              onBlur={() => { setIsRecordingCancelHotkey(false); setInputCancelHotkey(currentCancelHotkey); }}
              onKeyDown={(e) => handleKeyDown(e, true)}
              onKeyUp={(e) => handleKeyUp(e, true)}
            >
              <span className={`text-sm font-medium tracking-wide ${isRecordingCancelHotkey ? 'text-accent' : 'text-primary'}`}>
                {isRecordingCancelHotkey ? (inputCancelHotkey ? formatHotkey(inputCancelHotkey) : "...") : formatHotkey(currentCancelHotkey)}
              </span>
            </div>
            <button 
              onClick={handleResetCancelHotkey}
              className="text-secondary opacity-50 hover:opacity-100 transition-opacity"
              title="Reset to default (Escape)"
            >
              <RotateCcw className="w-4 h-4" />
            </button>
          </div>
        </div>

      </div>

      <div className="mt-8 animate-in fade-in slide-in-from-bottom-3 duration-300">
        <h3 className="text-xs font-bold text-secondary/70 uppercase tracking-wider mb-3 px-1">
          {t(lang, "controls.text_input_title")}
        </h3>
        <div className="bg-surface border border-border rounded-2xl flex flex-col">
          
          <div className="flex items-center justify-between p-5 border-b border-border">
            <div className="flex flex-col gap-1 w-1/3 pr-4">
              <span className="text-sm font-medium text-primary whitespace-nowrap">{t(lang, "controls.text_input_method")}</span>
            </div>
            <div className="flex items-center gap-3 w-2/3 justify-end">
              <select 
                value={textInputMethod}
                onChange={(e) => updateTextInputSetting("text_input_method", e.target.value)}
                className="w-full max-w-[280px] h-9 px-3 bg-window border border-border rounded-xl text-primary text-sm font-medium hover:border-border/80 focus:border-accent focus:outline-none transition-colors appearance-none cursor-pointer"
              >
                <option value="paste" title={""}>{t(lang, "controls.text_input_paste")}</option>
                <option value="paste_raw" title={t(lang, "controls.text_input_paste_raw_tooltip")}>{t(lang, "controls.text_input_paste_raw")}</option>
                <option value="paste_shift_ins" title={t(lang, "controls.text_input_paste_shift_ins_tooltip")}>{t(lang, "controls.text_input_paste_shift_ins")}</option>
                <option value="type_chars" title={t(lang, "controls.text_input_type_chars_tooltip")}>{t(lang, "controls.text_input_type_chars")}</option>
                <option value="copy_only" title={t(lang, "controls.text_input_copy_only_tooltip")}>{t(lang, "controls.text_input_copy_only")}</option>
              </select>
            </div>
          </div>
          
          <div className="flex items-center justify-between p-5 border-b border-border">
            <div className="flex items-center gap-2 group w-1/3">
              <span className="text-sm font-medium text-primary whitespace-nowrap">{t(lang, "controls.clipboard_after")}</span>
              <div className="relative flex items-center justify-center">
                <Info className="w-4 h-4 text-secondary opacity-50 cursor-help transition-opacity group-hover:opacity-100" />
                <div className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2 px-3 py-2 bg-surface border border-border text-primary text-xs rounded-lg w-72 text-center opacity-0 pointer-events-none transition-opacity group-hover:opacity-100 shadow-xl z-10 leading-relaxed">
                  {t(lang, "controls.clipboard_tooltip")}
                </div>
              </div>
            </div>
            <div className="flex items-center gap-3 w-2/3 justify-end">
              <select 
                value={clipboardAfter}
                onChange={(e) => updateTextInputSetting("clipboard_after", e.target.value)}
                disabled={textInputMethod === "copy_only"}
                className="w-full max-w-[280px] h-9 px-3 bg-window border border-border rounded-xl text-primary text-sm font-medium hover:border-border/80 focus:border-accent focus:outline-none transition-colors appearance-none cursor-pointer disabled:opacity-50"
              >
                <option value="restore">{t(lang, "controls.clipboard_restore")}</option>
                <option value="keep">{t(lang, "controls.clipboard_keep")}</option>
              </select>
            </div>
          </div>

          <div className="flex items-center justify-between p-5 border-b border-border">
            <div className="flex items-center gap-2 group w-1/3">
              <span className="text-sm font-medium text-primary whitespace-nowrap">{t(lang, "controls.send_after")}</span>
              <div className="relative flex items-center justify-center">
                <Info className="w-4 h-4 text-secondary opacity-50 cursor-help transition-opacity group-hover:opacity-100" />
                <div className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2 px-3 py-1.5 bg-surface border border-border text-primary text-xs rounded-lg w-48 text-center opacity-0 pointer-events-none transition-opacity group-hover:opacity-100 shadow-xl z-10">
                  {t(lang, "controls.send_after_tooltip")}
                </div>
              </div>
            </div>
            <div className="flex items-center gap-3 w-2/3 justify-end">
              <select 
                value={sendAfter}
                onChange={(e) => updateTextInputSetting("send_after", e.target.value)}
                disabled={textInputMethod === "copy_only"}
                className="w-full max-w-[280px] h-9 px-3 bg-window border border-border rounded-xl text-primary text-sm font-medium hover:border-border/80 focus:border-accent focus:outline-none transition-colors appearance-none cursor-pointer disabled:opacity-50"
              >
                <option value="none">{t(lang, "controls.send_after_none")}</option>
                <option value="enter">{t(lang, "controls.send_after_enter")}</option>
                <option value="ctrl_enter">{t(lang, "controls.send_after_ctrl_enter")}</option>
              </select>
            </div>
          </div>
          
          <div className="flex items-center justify-between p-5 cursor-pointer" onClick={() => updateTextInputSetting("trailing_space", !trailingSpace)}>
            <div className="flex flex-col gap-1">
              <span className="text-sm font-medium text-primary">
                {t(lang, "controls.trailing_space")}
              </span>
            </div>
            <div className={`w-11 h-6 rounded-full p-1 transition-colors ${trailingSpace ? 'bg-accent' : 'bg-border'}`}>
              <div className={`w-4 h-4 bg-knob shadow-sm rounded-full transition-transform ${trailingSpace ? 'translate-x-5' : 'translate-x-0'}`} />
            </div>
          </div>
          
        </div>
      </div>

      {activeModel && activeSettings && (
        <div className="mt-8 animate-in fade-in slide-in-from-bottom-2 duration-300">
          <h3 className="text-xs font-bold text-secondary/70 uppercase tracking-wider mb-3 px-1">
            {t(lang, "controls.model_settings_title", { modelName: activeModel.name })}
          </h3>
          <div className="bg-surface border border-border rounded-2xl flex flex-col">
            
            {/* Language Row */}
            <div className={`flex items-center justify-between p-5 ${isWhisper ? 'border-b border-border' : ''}`}>
              <div className="flex items-center gap-2 group">
                <span className="text-sm font-medium text-primary">{t(lang, "controls.language")}</span>
                <div className="relative flex items-center justify-center">
                  <Info className="w-4 h-4 text-secondary opacity-50 cursor-help transition-opacity group-hover:opacity-100" />
                  <div className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2 px-3 py-1.5 bg-surface border border-border text-primary text-xs rounded-lg whitespace-nowrap opacity-0 pointer-events-none transition-opacity group-hover:opacity-100 shadow-xl z-10">
                    {t(lang, "controls.language_tooltip")}
                  </div>
                </div>
              </div>
              <div className="flex items-center gap-3">
                {activeModel.id === "gigaam" ? (
                  <span className="text-sm font-medium text-secondary/60 italic pr-2">{t(lang, "controls.only_russian")}</span>
                ) : (
                  <>
                    <select 
                      value={activeSettings.language || "auto"}
                      onChange={(e) => saveModelSetting(activeModel.id, "language", e.target.value)}
                      className="h-9 px-3 bg-window border border-border rounded-xl text-primary text-sm font-medium hover:border-border/80 focus:border-accent focus:outline-none transition-colors appearance-none cursor-pointer"
                    >
                      {renderLanguageOptions().map(opt => (
                        <option key={opt.value} value={opt.value}>{opt.label}</option>
                      ))}
                    </select>
                    <button 
                      onClick={() => saveModelSetting(activeModel.id, "language", activeModel.id === "canary" ? "en" : "auto")}
                      className="text-secondary opacity-50 hover:opacity-100 transition-opacity"
                      title="Reset to default"
                    >
                      <RotateCcw className="w-4 h-4" />
                    </button>
                  </>
                )}
              </div>
            </div>

            {/* Initial Prompt Row (Whisper only) */}
            {isWhisper && (
              <div className="flex items-center justify-between p-5">
                <div className="flex items-center gap-2 group w-1/3">
                  <span className="text-sm font-medium text-primary whitespace-nowrap">{t(lang, "controls.initial_prompt")}</span>
                  <div className="relative flex items-center justify-center">
                    <Info className="w-4 h-4 text-secondary opacity-50 cursor-help transition-opacity group-hover:opacity-100" />
                    <div className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2 px-3 py-1.5 bg-surface border border-border text-primary text-xs rounded-lg w-48 text-center opacity-0 pointer-events-none transition-opacity group-hover:opacity-100 shadow-xl z-10">
                      {t(lang, "controls.initial_prompt_tooltip")}
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-3 w-2/3 justify-end">
                  <input 
                    type="text"
                    value={activeSettings.initial_prompt || ""}
                    onChange={(e) => saveModelSetting(activeModel.id, "initial_prompt", e.target.value)}
                    placeholder={t(lang, "controls.initial_prompt_placeholder")}
                    className="w-full max-w-[240px] h-9 px-3 bg-window border border-border rounded-xl text-primary text-sm focus:border-accent focus:outline-none transition-colors placeholder:text-secondary/40"
                  />
                  <button 
                    onClick={() => saveModelSetting(activeModel.id, "initial_prompt", null)}
                    className="text-secondary opacity-50 hover:opacity-100 transition-opacity"
                    title="Clear"
                  >
                    <RotateCcw className="w-4 h-4" />
                  </button>
                </div>
              </div>
            )}
            
          </div>
        </div>
      )}

      {error && (
        <div className="mt-4 p-3 bg-accent/10 border border-accent/20 rounded-xl text-accent text-sm flex items-center gap-2">
          <Info className="w-4 h-4" />
          {error.includes("conflict") ? t(lang, "controls.conflict") : error}
        </div>
      )}
    </div>
  );
}
