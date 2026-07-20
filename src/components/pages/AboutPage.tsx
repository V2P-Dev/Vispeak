import { Language, t } from "../../i18n";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useState } from "react";
import { useUpdater, UpdateStatus } from "../UpdaterProvider";
import { Logo } from "../Logo";

interface AboutPageProps {
  lang: Language;
}

export function AboutPage({ lang }: AboutPageProps) {
  const { currentVersion, isChecking, checkForUpdates } = useUpdater();
  const [checkStatus, setCheckStatus] = useState<UpdateStatus | "idle">("idle");

  const handleCheck = async () => {
    setCheckStatus("idle");
    const result = await checkForUpdates();
    if (result) {
      setCheckStatus(result.status);
    }
  };

  return (
    <div className="w-full max-w-2xl animate-in fade-in slide-in-from-bottom-4 duration-300">
      <div className="mb-8">
        <h2 className="text-3xl font-bold text-primary tracking-tight mb-2">
          {t(lang, "about.title")}
        </h2>
        <p className="text-secondary text-base max-w-2xl">
          {t(lang, "about.subtitle")}
        </p>
      </div>

      <div className="flex flex-col gap-6 p-8 bg-surface border border-border rounded-2xl items-center text-center">
        {/* Big Logo */}
        <div className="relative mb-2">
          <div className="w-24 h-24 flex items-center justify-center relative z-10">
            <Logo className="w-24 h-24 text-accent" />
          </div>
          {/* Subtle glow behind logo */}
          <div className="absolute inset-0 bg-accent/20 blur-2xl rounded-full z-0 translate-y-2 scale-75" />
        </div>

        <div className="flex flex-col gap-1">
          <h3 className="text-2xl font-bold text-primary">Vispeak</h3>
          <span className="text-sm font-medium text-secondary">
            {t(lang, "about.version")} {currentVersion}
          </span>
        </div>

        <div className="flex flex-col items-center gap-2 mt-2 w-full max-w-xs">
          <button 
            onClick={handleCheck}
            disabled={isChecking}
            className="w-full py-2.5 px-4 bg-window border border-border rounded-xl text-primary text-sm font-medium hover:border-primary transition-colors disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer"
          >
            {isChecking 
              ? (lang === "ru" ? "Проверка..." : "Checking...") 
              : (lang === "ru" ? "Проверить обновления" : "Check for updates")}
          </button>
          
          {checkStatus !== "idle" && (
            <div className="flex flex-col items-center mt-2">
              <span className={`text-xs font-medium text-center ${checkStatus === "up-to-date" || checkStatus === "available" ? "text-success" : "text-accent"}`}>
                {t(lang, `updater_status.${checkStatus}`)}
              </span>
            </div>
          )}
        </div>

        <div className="flex items-center gap-2 px-4 py-2 bg-success/10 border border-success/20 rounded-full text-success text-sm font-medium mt-2">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
          </svg>
          {t(lang, "about.offline")}
        </div>

        <div className="w-full h-px bg-border/50 my-2" />

        <div className="text-xs text-secondary/70 flex flex-col gap-1 text-left bg-window p-4 rounded-xl border border-border w-full mt-2">
          <p className="font-semibold text-secondary mb-1">
            {t(lang, "about.attribution")}
          </p>
          <p>• <b>Whisper</b>: MIT License (OpenAI)</p>
          <p>• <b>Parakeet & Canary</b>: CC-BY-4.0 (NVIDIA)</p>
          <p>• <b>GigaAM</b>: MIT License (SberDevices)</p>
        </div>

        <div className="text-xs text-secondary/70 flex flex-col gap-2 text-left bg-window p-4 rounded-xl border border-border w-full mt-2">
          <p className="font-semibold text-secondary mb-1">
            {t(lang, "about.privacy_title")}
          </p>
          <p>• {t(lang, "about.privacy_local")}</p>
          <p>• {t(lang, "about.privacy_history")}</p>
          <p>• {t(lang, "about.privacy_network")}</p>
        </div>

        <button 
          className="px-6 py-2.5 bg-window border border-border rounded-xl text-primary text-sm font-medium hover:border-primary transition-colors flex items-center gap-2 cursor-pointer"
          onClick={() => openUrl("https://github.com/V2P-Dev/Vispeak")}
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22" />
          </svg>
          {t(lang, "about.github")}
        </button>
      </div>
    </div>
  );
}
