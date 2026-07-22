import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Sidebar } from "./Sidebar";
import { ModelPage, ModelInfo } from "./pages/ModelPage";
import { ControlsPage } from "./pages/ControlsPage";
import { GeneralPage } from "./pages/GeneralPage";
import { AboutPage } from "./pages/AboutPage";
import { HistoryPage } from "./pages/HistoryPage";
import { getLanguage, Language, t } from "../i18n";
import { formatHotkey } from "../utils";
import { UpdateDialog } from "./UpdateDialog";
import { useUpdater } from "./UpdaterProvider";

type Page = "model" | "controls" | "history" | "general" | "about";

interface DownloadProgress {
  id: string;
  progress: number;
}

export function MainWindow() {
  const [activePage, setActivePage] = useState<Page>("model");
  const [lang, setLang] = useState<Language>("en");
  const scrollRef = useRef<HTMLDivElement>(null);
  
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [progressMap, setProgressMap] = useState<Record<string, number>>({});
  const [downloading, setDownloading] = useState<Record<string, boolean>>({});
  const [currentHotkey, setCurrentHotkey] = useState("Ctrl+Space");
  
  const [showUpdateDialog, setShowUpdateDialog] = useState(false);
  const { update } = useUpdater();

  const [modelStatus, setModelStatus] = useState<"loaded" | "unloaded" | "loading">("loaded");

  const fetchModels = async () => {
    const data = await invoke<ModelInfo[]>("get_models");
    setModels(data);
    
    // Check if onboarding needed
    if (data.length > 0 && data.every(m => !m.is_active)) {
      setActivePage("model");
    }
  };

  const fetchSettings = async () => {
    const s = await invoke<any>("get_settings");
    setCurrentHotkey(s.hotkey);
  };

  useEffect(() => {
    invoke<any>("get_settings").then(s => {
      setLang(getLanguage(s.app_language));
    });

    invoke<string>("get_model_status").then(status => {
      if (status === "loaded" || status === "unloaded" || status === "loading") {
        setModelStatus(status);
      }
    });

    fetchModels();

    const unlistenPromise = listen<DownloadProgress>("download-progress", (event) => {
      const { id, progress } = event.payload;
      setProgressMap(prev => ({ ...prev, [id]: progress }));
      
      if (progress >= 100) {
        setDownloading(prev => ({ ...prev, [id]: false }));
        fetchModels();
      }
    });

    const unlistenLoaded = listen("model-loaded", () => setModelStatus("loaded"));
    const unlistenUnloaded = listen("model-unloaded", () => setModelStatus("unloaded"));
    const unlistenLoading = listen("model-loading", () => setModelStatus("loading"));

    return () => {
      unlistenPromise.then(f => f());
      unlistenLoaded.then(f => f());
      unlistenUnloaded.then(f => f());
      unlistenLoading.then(f => f());
    };
  }, []);

  // Sync tray tooltip with active model and model status
  useEffect(() => {
    const activeModel = models.find(m => m.is_active);
    let statusText = "";
    if (modelStatus === "loaded") statusText = t(lang, "header.ready");
    else if (modelStatus === "unloaded") statusText = t(lang, "header.standby");
    else if (modelStatus === "loading") statusText = t(lang, "header.loading");

    const tooltip = activeModel
      ? `${activeModel.name} — ${statusText}`
      : t(lang, "errors.err_no_model_selected");

    invoke("update_tray_tooltip", { tooltip }).catch(console.error);
  }, [models, modelStatus, lang]);

  // Re-fetch settings when active page changes, or periodically if we wanted to
  useEffect(() => {
    fetchSettings();
    const interval = setInterval(fetchSettings, 2000); // Poll every 2 seconds to keep header in sync
    return () => clearInterval(interval);
  }, [activePage]);

  // Reset scroll position when switching pages
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = 0;
    }
  }, [activePage]);

  const handleDownload = async (id: string) => {
    setDownloading(prev => ({ ...prev, [id]: true }));
    setProgressMap(prev => ({ ...prev, [id]: 0 }));
    try {
      await invoke("download_model", { id });
      fetchModels();
    } catch (e) {
      console.error(e);
      setDownloading(prev => ({ ...prev, [id]: false }));
    }
  };

  const handleActivate = async (id: string) => {
    await invoke("set_active_model", { modelId: id });
    fetchModels();
  };

  return (
    <div className="flex h-screen w-full bg-window text-primary overflow-hidden">
      {showUpdateDialog && update && (
        <UpdateDialog 
          update={update} 
          lang={lang} 
          onClose={() => setShowUpdateDialog(false)} 
        />
      )}
      <Sidebar 
        activePage={activePage} 
        onPageChange={setActivePage} 
        lang={lang} 
        onShowUpdate={() => setShowUpdateDialog(true)} 
      />
      
      <div className="flex-1 flex flex-col relative overflow-hidden">
        {/* Top Header */}
        <div className="h-20 flex items-center justify-end px-8 shrink-0 w-full z-10">
          <div className="flex items-center gap-4">
            <div className="flex items-center gap-2 text-sm font-medium text-secondary">
              {modelStatus === "loaded" && t(lang, "header.ready")}
              {modelStatus === "unloaded" && t(lang, "header.standby")}
              {modelStatus === "loading" && t(lang, "header.loading")}
              <span 
                className={`w-2 h-2 rounded-full transition-all duration-300 ${
                  modelStatus === "loaded"
                    ? "bg-success shadow-[0_0_8px_rgba(126,212,145,0.6)]"
                    : modelStatus === "loading"
                    ? "bg-processing shadow-[0_0_8px_rgba(77,216,230,0.6)] animate-pulse"
                    : "bg-secondary/60 shadow-none"
                }`}
              />
            </div>
            
            <div className="px-3 py-1.5 border border-border/80 rounded-full text-xs font-semibold tracking-wider text-secondary bg-surface shadow-sm">
              {formatHotkey(currentHotkey)}
            </div>
          </div>
        </div>

        {/* Page Content */}
        <div ref={scrollRef} className="flex-1 overflow-y-auto px-10 pb-10">
          {activePage === "model" && (
            <ModelPage 
              lang={lang}
              models={models}
              downloading={downloading}
              progressMap={progressMap}
              onDownload={handleDownload}
              onActivate={handleActivate}
            />
          )}
          {activePage === "controls" && <ControlsPage lang={lang} />}
          {activePage === "history" && <HistoryPage lang={lang} />}
          {activePage === "general" && <GeneralPage lang={lang} />}
          {activePage === "about" && <AboutPage lang={lang} />}
        </div>
      </div>
    </div>
  );
}
