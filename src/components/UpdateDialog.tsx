import { useState } from "react";
import { Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Language, t } from "../i18n";

interface UpdateDialogProps {
  update: Update;
  lang: Language;
  onClose: () => void;
}

export function UpdateDialog({ update, lang, onClose }: UpdateDialogProps) {
  const [downloading, setDownloading] = useState(false);
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const startUpdate = async () => {
    setDownloading(true);
    setError(null);
    let downloaded = 0;
    let contentLength = 0;

    try {
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            contentLength = event.data.contentLength || 0;
            break;
          case 'Progress':
            downloaded += event.data.chunkLength;
            if (contentLength > 0) {
              setProgress(Math.round((downloaded / contentLength) * 100));
            }
            break;
          case 'Finished':
            setProgress(100);
            break;
        }
      });
      
      // After install, restart the app
      await relaunch();
    } catch (e) {
      console.error("Update failed:", e);
      setError(String(e));
      setDownloading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm animate-in fade-in duration-200" onClick={!downloading ? onClose : undefined}>
      <div 
        className="bg-surface border border-border w-[400px] max-w-[90vw] rounded-2xl shadow-xl overflow-hidden flex flex-col"
        onClick={e => e.stopPropagation()}
      >
        <div className="p-6 pb-4">
          <h3 className="text-xl font-bold text-primary mb-1">
            {lang === "ru" ? `Обновление v${update.version}` : `Update v${update.version}`}
          </h3>
          <div className="text-sm text-secondary">
            {update.body ? (
              <div className="max-h-32 overflow-y-auto pr-2 custom-scrollbar text-xs border border-border/50 bg-window rounded-lg p-3 mt-3">
                {update.body}
              </div>
            ) : (
              <p className="mt-2">{lang === "ru" ? "Доступна новая версия Vispeak." : "A new version of Vispeak is available."}</p>
            )}
            <p className="text-xs mt-3">
              {t(lang, "updater_status.see_github")}
              <button 
                className="text-accent hover:underline cursor-pointer transition-colors"
                onClick={() => openUrl(`https://github.com/V2P-Dev/Vispeak/releases/tag/v${update.version}`)}
              >
                {t(lang, "updater_status.release_notes_github")}
              </button>
            </p>
          </div>
        </div>

        {error && (
          <div className="px-6 py-3 bg-accent/10 border-y border-accent/20 text-accent text-sm">
            {lang === "ru" ? "Ошибка обновления: " : "Update error: "}{error}
          </div>
        )}

        <div className="p-6 pt-4 bg-window/50 border-t border-border flex justify-end gap-3">
          {!downloading ? (
            <>
              <button
                onClick={onClose}
                className="px-4 py-2 rounded-xl text-sm font-medium text-secondary hover:text-primary hover:bg-surface border border-transparent transition-colors cursor-pointer"
              >
                {lang === "ru" ? "Позже" : "Later"}
              </button>
              <button
                onClick={startUpdate}
                className="px-4 py-2 rounded-xl text-sm font-medium bg-accent text-black hover:bg-accent/90 shadow-[var(--shadow-accent-sm)] transition-all cursor-pointer"
              >
                {lang === "ru" ? "Обновить сейчас" : "Update now"}
              </button>
            </>
          ) : (
            <div className="w-full flex flex-col gap-2">
              <div className="flex justify-between text-xs text-secondary font-medium">
                <span>{lang === "ru" ? "Скачивание..." : "Downloading..."}</span>
                <span>{progress}%</span>
              </div>
              <div className="w-full h-1.5 bg-surface rounded-full overflow-hidden border border-border/50">
                <div 
                  className="h-full bg-accent transition-all duration-300 shadow-[0_0_8px_rgba(255,85,51,0.6)]"
                  style={{ width: `${progress}%` }}
                />
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
