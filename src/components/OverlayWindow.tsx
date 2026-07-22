// The capsule is displayed on top of arbitrary desktop windows; status colors (red/cyan/green) are part of the core product language.
import { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getLanguage, t, Language } from "../i18n";

type AppInfo = {
  title: string;
  icon_base64: string;
};

function Visualizer({ level, colorClass = "bg-accent", pulse = false, count = 52, maxHeight = 24 }: { level: number, colorClass?: string, pulse?: boolean, count?: number, maxHeight?: number }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const levelRef = useRef(level);
  const pulseRef = useRef(pulse);

  useEffect(() => {
    levelRef.current = level;
  }, [level]);

  useEffect(() => {
    pulseRef.current = pulse;
  }, [pulse]);

  useEffect(() => {
    let animationFrameId: number;
    const currentHeights = new Array(count).fill(3);
    let lastTime = performance.now();

    const update = (now: number) => {
      const dt = Math.min((now - lastTime) / 1000, 0.1);
      lastTime = now;
      const timeSec = now / 1000;

      const rawLevel = levelRef.current;
      const scaledLevel = Math.min(Math.pow(rawLevel, 0.4) * 2.5, 1.0);
      const isPulse = pulseRef.current;

      const maxBar = maxHeight || 24;
      const container = containerRef.current;

      if (container && container.children.length === count) {
        for (let i = 0; i < count; i++) {
          // Asymmetric phase variation across bars without symmetrical envelope
          const noise1 = Math.sin(timeSec * 4.5 + i * 0.73);
          const noise2 = Math.cos(timeSec * 2.8 - i * 0.41);
          const variation = 0.5 + 0.5 * ((noise1 + noise2) * 0.5);

          // Living baseline in silence (small 1-2px jitter)
          const livingBase = isPulse ? 0 : 0.04 + 0.05 * Math.sin(timeSec * 3.0 + i * 1.3);

          const activeLevel = isPulse ? 0.35 + 0.15 * variation : Math.max(livingBase, scaledLevel * (0.55 + 0.45 * variation));
          const targetHeight = 3 + (maxBar - 3) * activeLevel;

          // Asymmetric smoothing: fast attack (rise), slow decay (fall)
          const current = currentHeights[i];
          const speed = targetHeight > current ? 18.0 : 5.0;
          currentHeights[i] = current + (targetHeight - current) * Math.min(1.0, speed * dt);

          const el = container.children[i] as HTMLElement;
          if (el) {
            el.style.height = `${currentHeights[i].toFixed(2)}px`;
          }
        }
      }

      animationFrameId = requestAnimationFrame(update);
    };

    animationFrameId = requestAnimationFrame(update);
    return () => cancelAnimationFrame(animationFrameId);
  }, [count, maxHeight]);

  return (
    <div ref={containerRef} className={`flex items-center justify-center gap-0.5 h-full w-full ${pulse ? 'animate-pulse opacity-80' : ''}`}>
      {Array.from({ length: count }).map((_, i) => (
        <div 
          key={i}
          className={`w-1 rounded-full ${colorClass}`}
          style={{ height: '3px' }}
        />
      ))}
    </div>
  );
}

function useOverlayState() {
  const [level, setLevel] = useState(0);
  const [statusText, setStatusText] = useState("");
  const [errorText, setErrorText] = useState<string | null>(null);
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);

  const [isRecording, setIsRecording] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);
  const [isSuccess, setIsSuccess] = useState(false);
  const [isCopied, setIsCopied] = useState(false);
  const [isError, setIsError] = useState(false);
  
  const [lang, setLang] = useState<Language>("en");
  const [skin, setSkin] = useState<"full" | "compact" | "mini" | string>("full");

  useEffect(() => {
    invoke<any>("get_settings").then(settings => {
      setLang(getLanguage(settings.app_language));
      if (settings.overlay_skin) {
        setSkin(settings.overlay_skin);
      }
    });

    const unlistenLevel = listen<number>("audio-level", (event) => {
      setLevel(event.payload);
    });

    const unlistenAppInfo = listen<AppInfo>("target-app", (event) => {
      setAppInfo(event.payload);
    });

    const unlistenModelLoading = listen("model-loading", () => {
      setStatusText(t(lang, "overlay.loading_model"));
    });

    const unlistenModelLoaded = listen("model-loaded", () => {
      setStatusText(t(lang, "overlay.recording"));
    });
    
    const unlistenStarted = listen("recording-started", () => {
      setStatusText(t(lang, "overlay.recording")); // In compact, text is "Listening", we can override in component
      setIsRecording(true);
      setIsProcessing(false);
      setIsSuccess(false);
      setIsError(false);
      setErrorText(null);
    });

    const unlistenProcessing = listen("processing-started", () => {
      setStatusText(t(lang, "overlay.processing"));
      setIsRecording(false);
      setIsProcessing(true);
      setIsSuccess(false);
      setIsError(false);
    });

    const unlistenDone = listen<string>("transcription-done", (event) => {
      const payload = event.payload;
      if (payload.startsWith("Error: ")) {
        const code = payload.replace("Error: ", "");
        // "err_speech_not_recognized" usually just shows silently without red error if user canceled
        if (code === "err_speech_not_recognized") {
          setErrorText(null);
        } else {
          setErrorText(t(lang, `errors.${code}`));
        }
        setIsRecording(false);
        setIsProcessing(false);
        setIsSuccess(false);
        setIsCopied(false);
        setIsError(true);
      } else if (payload.startsWith("COPIED:")) {
        setStatusText(t(lang, "overlay.copied"));
        setIsRecording(false);
        setIsProcessing(false);
        setIsSuccess(true);
        setIsCopied(true);
        setIsError(false);
      } else {
        setStatusText(payload);
        setIsRecording(false);
        setIsProcessing(false);
        setIsSuccess(true);
        setIsCopied(false);
        setIsError(false);
      }
      
      setTimeout(() => {
        invoke("hide_overlay");
      }, 1500);
    });

    const unlistenError = listen<string>("show-error", (event) => {
      // event.payload is now an error code, e.g. "err_mic_not_found"
      setErrorText(t(getLanguage(lang), `errors.${event.payload}`));
      setIsRecording(false);
      setIsProcessing(false);
      setIsSuccess(false);
      setIsError(true);
      
      setTimeout(() => {
        invoke("hide_overlay");
        setErrorText(null);
      }, 2000);
    });

    const unlistenCancelled = listen("recording-cancelled", () => {
      setIsRecording(false);
      setIsProcessing(false);
      setIsSuccess(false);
      setIsError(false);
    });

    const unlistenCancelledSilently = listen("recording-cancelled-silently", () => {
      setIsRecording(false);
      setIsProcessing(false);
      setIsSuccess(false);
      setIsError(false);
    });

    const unlistenSettings = listen("settings-updated", () => {
       invoke<any>("get_settings").then(settings => {
         if (settings.overlay_skin) {
           setSkin(settings.overlay_skin);
         }
       });
    });

    return () => {
      unlistenLevel.then(f => f());
      unlistenAppInfo.then(f => f());
      unlistenModelLoading.then(f => f());
      unlistenModelLoaded.then(f => f());
      unlistenStarted.then(f => f());
      unlistenProcessing.then(f => f());
      unlistenDone.then(f => f());
      unlistenError.then(f => f());
      unlistenCancelled.then(f => f());
      unlistenCancelledSilently.then(f => f());
      unlistenSettings.then(f => f());
    };
  }, [lang]);

  return { level, statusText, errorText, appInfo, isRecording, isProcessing, isSuccess, isCopied, isError, lang, skin };
}

function OverlayFull(props: ReturnType<typeof useOverlayState>) {
  const { level, statusText, errorText, appInfo, isRecording, isProcessing, isSuccess, isError, lang } = props;

  let glowClass = "border-border/50 shadow-lg";
  let footerText = t(lang, "overlay.cancel");
  if (isRecording) {
    glowClass = "border-accent/40 animate-glow-pulse";
  } else if (isProcessing) {
    glowClass = "border-processing/40 shadow-[0_0_24px_rgba(77,216,230,0.3)]";
    footerText = t(lang, "overlay.processing");
  } else if (isSuccess) {
    glowClass = "border-success/40 shadow-[0_0_24px_rgba(126,212,145,0.3)]";
  } else if (isError) {
    glowClass = "border-accent/80 shadow-[0_0_24px_rgba(255,85,51,0.15)]";
  }

  return (
    <div className="flex w-full h-full items-center justify-center p-6 bg-transparent">
      <div className={`w-[344px] h-[114px] bg-overlay/90 backdrop-blur-md rounded-[20px] flex flex-col border transition-all duration-300 ${glowClass} overflow-hidden`}>
        
        {/* Header */}
        <div className="flex flex-row items-center justify-between w-full px-3 py-1.5 border-b border-border/50 bg-surface/50">
          <div className="flex flex-row items-center gap-2 overflow-hidden flex-1">
            {appInfo?.icon_base64 ? (
              <img src={`data:image/png;base64,${appInfo.icon_base64}`} className="w-3.5 h-3.5 object-contain" />
            ) : (
              <div className="w-3.5 h-3.5 bg-secondary/20 rounded-sm"></div>
            )}
            <span className="text-primary font-semibold text-xs truncate pr-2">
              {appInfo?.title || "Vispeak"}
            </span>
          </div>
          <div className="flex flex-row items-center gap-1 opacity-50 shrink-0">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-3 h-3 text-secondary" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M7.9 20A9 9 0 1 0 4 16.1L2 22Z" strokeWidth="2"/>
              <path d="M8 11v2" strokeWidth="1.5"/>
              <path d="M10 9v6" strokeWidth="1.5"/>
              <path d="M12 7v10" strokeWidth="1.5"/>
              <path d="M14 9v6" strokeWidth="1.5"/>
              <path d="M16 11v2" strokeWidth="1.5"/>
            </svg>
            <span className="text-secondary text-[10px]">Vispeak</span>
          </div>
        </div>

        {/* Center */}
        <div className="flex-1 w-full flex items-center justify-center px-4 overflow-hidden relative">
          {isRecording && <Visualizer level={level} colorClass="bg-accent" />}
          {isProcessing && <Visualizer level={0.0} colorClass="bg-processing" pulse />}
          {isSuccess && (
            <span className="text-primary text-sm tracking-wide text-center line-clamp-2 leading-tight">
              {statusText}
            </span>
          )}
          {isError && (
            <span className="text-accent text-sm font-medium text-center">
              {errorText || t(lang, "overlay.error_no_mic")}
            </span>
          )}
        </div>

        {/* Footer */}
        <div className="w-full text-center py-1 border-t border-border/50 bg-surface/50">
          <span className="text-secondary text-[10px]">
            {footerText}
          </span>
        </div>

      </div>
    </div>
  );
}

function OverlayCompact(props: ReturnType<typeof useOverlayState>) {
  const { level, errorText, isRecording, isProcessing, isSuccess, isCopied, isError, lang } = props;
  
  const containerRef = useRef<HTMLDivElement>(null);
  const [dotCount, setDotCount] = useState(18);

  useEffect(() => {
    if (!containerRef.current) return;
    const observer = new ResizeObserver(entries => {
      for (let entry of entries) {
        const width = entry.contentRect.width;
        // Dot takes 4px (w-1) + gap takes 2px (gap-0.5).
        // Total width W = N * 4 + (N - 1) * 2 = 6N - 2
        // N = Math.floor((W + 2) / 6)
        setDotCount(Math.max(1, Math.floor((width + 2) / 6)));
      }
    });
    observer.observe(containerRef.current);
    return () => observer.disconnect();
  }, []);

  let glowClass = "border-border/50 shadow-lg";

  if (isRecording) {
    glowClass = "border-accent/40 animate-glow-pulse";
  } else if (isProcessing) {
    glowClass = "border-processing/40 shadow-[0_0_24px_rgba(77,216,230,0.3)]";
  } else if (isSuccess) {
    glowClass = "border-success/40 shadow-[0_0_24px_rgba(126,212,145,0.3)] animate-out slide-out-to-bottom-4 duration-500 delay-500";
  } else if (isError) {
    glowClass = "border-accent/80 shadow-[0_0_24px_rgba(255,85,51,0.15)]";
  }

  return (
    <div className="flex w-full h-full items-center justify-center p-6 bg-transparent">
      <div className={`w-[220px] h-[48px] px-4 bg-overlay/90 backdrop-blur-md rounded-full flex flex-row items-center border transition-all duration-300 ${glowClass} overflow-hidden relative`}>
        
        {/* RECORDING STATE */}
        <div className={`absolute inset-0 px-4 flex flex-row items-center transition-opacity duration-200 ${isRecording ? "opacity-100" : "opacity-0 pointer-events-none"}`}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4 text-accent shrink-0" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"></path>
            <path d="M19 10v2a7 7 0 0 1-14 0v-2"></path>
            <line x1="12" x2="12" y1="19" y2="22"></line>
          </svg>
          <div ref={containerRef} className="flex-1 flex justify-center items-center overflow-hidden pl-4 pr-0">
            <Visualizer level={level} colorClass="bg-accent" count={dotCount} />
          </div>
        </div>

        {/* PROCESSING STATE */}
        <div className={`absolute inset-0 px-4 flex flex-row items-center justify-center transition-opacity duration-200 ${isProcessing ? "opacity-100" : "opacity-0 pointer-events-none"}`}>
           <div className="flex flex-row items-center gap-1.5 mr-2">
             <div className="w-1.5 h-1.5 rounded-full bg-processing animate-processing-dot" style={{ animationDelay: '0ms' }}></div>
             <div className="w-1.5 h-1.5 rounded-full bg-processing animate-processing-dot" style={{ animationDelay: '150ms' }}></div>
             <div className="w-1.5 h-1.5 rounded-full bg-processing animate-processing-dot" style={{ animationDelay: '300ms' }}></div>
           </div>
           <span className="text-secondary text-xs font-medium truncate">{t(lang, "overlay.processing")}</span>
        </div>

        {/* SUCCESS STATE */}
        <div className={`absolute inset-0 flex flex-row items-center justify-center transition-opacity duration-200 ${isSuccess && !isCopied ? "opacity-100" : "opacity-0 pointer-events-none"}`}>
           <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-6 h-6 text-success" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
             <polyline points="20 6 9 17 4 12"></polyline>
           </svg>
        </div>

        {/* COPIED STATE */}
        <div className={`absolute inset-0 flex flex-row items-center justify-center transition-opacity duration-200 ${isSuccess && isCopied ? "opacity-100" : "opacity-0 pointer-events-none"}`}>
           <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-5 h-5 text-success" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
             <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
             <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
           </svg>
        </div>

        {/* ERROR STATE */}
        <div className={`absolute inset-0 px-4 flex flex-row items-center transition-opacity duration-200 ${isError ? "opacity-100" : "opacity-0 pointer-events-none"}`}>
           <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-5 h-5 text-accent shrink-0" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
             <line x1="18" y1="6" x2="6" y2="18"></line>
             <line x1="6" y1="6" x2="18" y2="18"></line>
           </svg>
           <span className="text-accent text-sm font-medium truncate flex-1 ml-2 text-center">
             {errorText || t(lang, "overlay.error_no_mic")}
           </span>
        </div>

      </div>
    </div>
  );
}

function OverlayMini(props: ReturnType<typeof useOverlayState>) {
  const { level, isRecording, isProcessing, isSuccess, isCopied, isError } = props;
  
  const containerRef = useRef<HTMLDivElement>(null);
  const [dotCount, setDotCount] = useState(13);

  useEffect(() => {
    if (!containerRef.current) return;
    const observer = new ResizeObserver(entries => {
      for (let entry of entries) {
        const width = entry.contentRect.width;
        setDotCount(Math.max(1, Math.floor((width + 2) / 6)));
      }
    });
    observer.observe(containerRef.current);
    return () => observer.disconnect();
  }, []);

  let glowClass = "border-border/50 shadow-lg";

  if (isRecording) {
    glowClass = "border-accent/40 animate-glow-pulse";
  } else if (isProcessing) {
    glowClass = "border-processing/40 shadow-[0_0_24px_rgba(77,216,230,0.3)]";
  } else if (isSuccess) {
    glowClass = "border-success/40 shadow-[0_0_24px_rgba(126,212,145,0.3)] animate-out slide-out-to-bottom-4 duration-500 delay-500";
  } else if (isError) {
    glowClass = "border-accent/80 shadow-[0_0_24px_rgba(255,85,51,0.15)]";
  }

  return (
    <div className="flex w-full h-full items-center justify-center p-6 bg-transparent">
      <div className={`w-[96px] h-[34px] px-2.5 bg-overlay/90 backdrop-blur-md rounded-full flex flex-row items-center justify-center border transition-all duration-300 ${glowClass} overflow-hidden relative`}>
        
        {/* RECORDING STATE */}
        <div className={`absolute inset-0 px-2.5 flex flex-row items-center justify-center transition-opacity duration-200 ${isRecording ? "opacity-100" : "opacity-0 pointer-events-none"}`}>
          <div ref={containerRef} className="w-full h-6 flex justify-center items-center overflow-hidden">
            <Visualizer level={level} colorClass="bg-accent" count={dotCount} maxHeight={22} />
          </div>
        </div>

        {/* PROCESSING STATE */}
        <div className={`absolute inset-0 flex flex-row items-center justify-center transition-opacity duration-200 ${isProcessing ? "opacity-100" : "opacity-0 pointer-events-none"}`}>
           <div className="flex flex-row items-center gap-2">
             <div className="w-2 h-2 rounded-full bg-processing animate-processing-dot" style={{ animationDelay: '0ms' }}></div>
             <div className="w-2 h-2 rounded-full bg-processing animate-processing-dot" style={{ animationDelay: '150ms' }}></div>
             <div className="w-2 h-2 rounded-full bg-processing animate-processing-dot" style={{ animationDelay: '300ms' }}></div>
           </div>
        </div>

        {/* SUCCESS STATE */}
        <div className={`absolute inset-0 flex flex-row items-center justify-center transition-opacity duration-200 ${isSuccess && !isCopied ? "opacity-100" : "opacity-0 pointer-events-none"}`}>
           <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4 text-success" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
             <polyline points="20 6 9 17 4 12"></polyline>
           </svg>
        </div>

        {/* COPIED STATE */}
        <div className={`absolute inset-0 flex flex-row items-center justify-center transition-opacity duration-200 ${isSuccess && isCopied ? "opacity-100" : "opacity-0 pointer-events-none"}`}>
           <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4 text-success" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
             <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
             <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
           </svg>
        </div>

        {/* ERROR STATE */}
        <div className={`absolute inset-0 flex flex-row items-center justify-center transition-opacity duration-200 ${isError ? "opacity-100" : "opacity-0 pointer-events-none"}`}>
           <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4 text-accent" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
             <line x1="18" y1="6" x2="6" y2="18"></line>
             <line x1="6" y1="6" x2="18" y2="18"></line>
           </svg>
        </div>

      </div>
    </div>
  );
}

export function OverlayWindow() {
  const state = useOverlayState();

  if (!state.isRecording && !state.isProcessing && !state.isSuccess && !state.isError) {
     return <div className="hidden"></div>;
  }

  if (state.skin === "mini") {
    return <OverlayMini {...state} />;
  }

  if (state.skin === "compact") {
    return <OverlayCompact {...state} />;
  }

  return <OverlayFull {...state} />;
}
