import { Language, t } from "../i18n";

type Page = "model" | "controls" | "history" | "general" | "about";

interface SidebarProps {
  activePage: Page;
  onPageChange: (page: Page) => void;
  lang: Language;
  onShowUpdate: () => void;
}

import type { ReactNode } from "react";
import { useUpdater } from "./UpdaterProvider";

export function Sidebar({ activePage, onPageChange, lang, onShowUpdate }: SidebarProps) {
  const { update, currentVersion } = useUpdater();
  const navItems: { id: Page; icon: ReactNode; label: string }[] = [
    {
      id: "model",
      label: t(lang, "sidebar.model"),
      icon: (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-5 h-5" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M22 12h-4l-3 9L9 3l-3 9H2" />
        </svg>
      )
    },
    {
      id: "controls",
      label: t(lang, "sidebar.controls"),
      icon: (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-5 h-5" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M12 20a8 8 0 1 0 0-16 8 8 0 0 0 0 16Z" />
          <path d="M12 14a2 2 0 1 0 0-4 2 2 0 0 0 0 4Z" />
          <path d="M12 2v2" />
          <path d="M12 22v-2" />
          <path d="m17 20.66-1-1.73" />
          <path d="M11 10.27 7 3.34" />
          <path d="m20.66 17-1.73-1" />
          <path d="m3.34 7 1.73 1" />
          <path d="M14 12h8" />
          <path d="M2 12h2" />
          <path d="m20.66 7-1.73 1" />
          <path d="m3.34 17 1.73-1" />
          <path d="m17 3.34-1 1.73" />
          <path d="m11 13.73-4 6.93" />
        </svg>
      )
    },
    {
      id: "history",
      label: t(lang, "sidebar.history"),
      icon: (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-5 h-5" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="10" />
          <polyline points="12 6 12 12 16 14" />
        </svg>
      )
    },
    {
      id: "general",
      label: t(lang, "sidebar.general"),
      icon: (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-5 h-5" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <line x1="21" x2="14" y1="4" y2="4" />
          <line x1="10" x2="3" y1="4" y2="4" />
          <line x1="21" x2="12" y1="12" y2="12" />
          <line x1="8" x2="3" y1="12" y2="12" />
          <line x1="21" x2="16" y1="20" y2="20" />
          <line x1="12" x2="3" y1="20" y2="20" />
          <line x1="14" x2="14" y1="2" y2="6" />
          <line x1="8" x2="8" y1="10" y2="14" />
          <line x1="16" x2="16" y1="18" y2="22" />
        </svg>
      )
    },
    {
      id: "about",
      label: t(lang, "sidebar.about"),
      icon: (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-5 h-5" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="10" />
          <path d="M12 16v-4" />
          <path d="M12 8h.01" />
        </svg>
      )
    }
  ];

  return (
    <div className="w-[220px] bg-surface border-r border-border flex flex-col h-full shrink-0">
      {/* Logo */}
      <div className="h-20 flex items-center px-6 shrink-0 relative">
        <h1 className="text-2xl font-bold tracking-tight text-primary flex items-center gap-2">
          Vispeak
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-6 h-6 text-accent drop-shadow-[var(--shadow-accent-sm)]" strokeLinecap="round" strokeLinejoin="round">
            <path d="M7.9 20A9 9 0 1 0 4 16.1L2 22Z" strokeWidth="2"/>
            <path d="M8 11v2" strokeWidth="1.5"/>
            <path d="M10 9v6" strokeWidth="1.5"/>
            <path d="M12 7v10" strokeWidth="1.5"/>
            <path d="M14 9v6" strokeWidth="1.5"/>
            <path d="M16 11v2" strokeWidth="1.5"/>
          </svg>
        </h1>
      </div>

      {/* Navigation */}
      <nav className="flex-1 px-3 py-2 flex flex-col gap-1 overflow-y-auto">
        {navItems.map((item) => {
          const isActive = activePage === item.id;
          return (
            <button
              key={item.id}
              onClick={() => onPageChange(item.id)}
              className={`flex items-center gap-3 px-3 py-2.5 rounded-[12px] transition-all duration-200 cursor-pointer relative ${
                isActive 
                  ? "bg-window text-primary" 
                  : "text-secondary hover:text-primary hover:bg-window/50"
              }`}
            >
              {isActive && (
                <div className="absolute left-0 top-1/2 -translate-y-1/2 w-0.5 h-5 bg-accent rounded-r-full shadow-[var(--shadow-accent-sm)]" />
              )}
              <div className={`${isActive ? "text-accent" : ""}`}>
                {item.icon}
              </div>
              <span className={`font-medium ${isActive ? "font-semibold" : ""}`}>
                {item.label}
              </span>
            </button>
          );
        })}
      </nav>

      {/* Footer / Updater */}
      <div className="p-4 border-t border-border shrink-0">
        {update ? (
          <button 
            onClick={onShowUpdate}
            className="w-full flex items-center justify-center gap-2 px-3 py-2 bg-accent text-black rounded-xl text-xs font-semibold shadow-[var(--shadow-accent-sm)] hover:bg-accent/90 transition-colors cursor-pointer"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="7 10 12 15 17 10" />
              <line x1="12" x2="12" y1="15" y2="3" />
            </svg>
            {lang === "ru" ? `Доступно v${update.version}` : `Update v${update.version}`}
          </button>
        ) : (
          <div 
            onClick={() => onPageChange("about")}
            className="w-full text-center text-xs font-medium text-secondary/60 hover:text-secondary cursor-pointer transition-colors"
          >
            v{currentVersion}
          </div>
        )}
      </div>
    </div>
  );
}
