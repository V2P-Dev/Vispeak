import { useState, useEffect, useRef } from "react";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Language, t } from "../../i18n";

interface HistoryRecord {
  id: number;
  timestamp: string;
  text: string;
  model_id: string;
  target_app_name: string;
  target_app_icon: string;
  duration_sec: number;
  has_audio: boolean;
}

interface HistoryPageProps {
  lang: Language;
}

export function HistoryPage({ lang }: HistoryPageProps) {
  const [records, setRecords] = useState<HistoryRecord[]>([]);
  const [models, setModels] = useState<Record<string, string>>({});
  const [retranscribing, setRetranscribing] = useState<Record<number, boolean>>({});
  const [copiedId, setCopiedId] = useState<number | null>(null);
  const [pastingId, setPastingId] = useState<number | null>(null);
  const [playingId, setPlayingId] = useState<number | null>(null);
  const [currentTime, setCurrentTime] = useState<number>(0);
  const [duration, setDuration] = useState<number>(0);
  const audioRef = useRef<HTMLAudioElement | null>(null);

  const fetchHistory = async () => {
    try {
      const data = await invoke<HistoryRecord[]>("get_history");
      setRecords(data);
    } catch (e) {
      console.error("Failed to fetch history", e);
    }
  };

  useEffect(() => {
    fetchHistory();
    invoke<any[]>("get_models").then(data => {
      const map: Record<string, string> = {};
      for (const m of data) map[m.id] = m.name;
      setModels(map);
    }).catch(console.error);
  }, []);

  useEffect(() => {
    const unlistenDone = listen<[number, string]>("retranscription-done", async (event) => {
      const [id, newText] = event.payload;
      setRetranscribing(prev => ({ ...prev, [id]: false }));
      
      const s = await invoke<any>("get_settings");
      await invoke("update_history_record_text", { id, newText, newModelId: s.active_model });
      
      fetchHistory();
    });

    const unlistenErr = listen<[number, string]>("retranscription-error", (event) => {
      const [id, err] = event.payload;
      setRetranscribing(prev => ({ ...prev, [id]: false }));
      console.error("Retranscription error", err);
    });

    return () => {
      unlistenDone.then(f => f());
      unlistenErr.then(f => f());
    };
  }, []);

  const handleCopy = (id: number, text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedId(id);
    setTimeout(() => {
      setCopiedId(prev => (prev === id ? null : prev));
    }, 1500);
  };

  const handleDelete = async (id: number) => {
    await invoke("delete_history_record", { id });
    fetchHistory();
  };

  const handleRetranscribe = async (id: number) => {
    setRetranscribing(prev => ({ ...prev, [id]: true }));
    try {
      await invoke("retranscribe_history_record", { id });
    } catch (e) {
      console.error(e);
      setRetranscribing(prev => ({ ...prev, [id]: false }));
    }
  };

  const handlePlay = async (id: number) => {
    console.log("[Play audio] Clicked play for ID:", id);
    const record = records.find(rec => rec.id === id);
    console.log("[Play audio] record.has_audio =", record?.has_audio);

    if (playingId === id) {
      if (audioRef.current) {
        audioRef.current.pause();
        audioRef.current.currentTime = 0;
      }
      setPlayingId(null);
      setCurrentTime(0);
      return;
    }

    try {
      const path = await invoke<string>("get_history_audio_path", { id });
      console.log("[Play audio] Path from backend:", path);
      const url = convertFileSrc(path);
      console.log("[Play audio] URL from convertFileSrc:", url);
      
      if (audioRef.current) {
        audioRef.current.src = url;
        setPlayingId(id);
        setCurrentTime(0);
        setDuration(record ? record.duration_sec : 0);
        audioRef.current.play().catch(err => {
          console.error("[Play audio] audioRef.current.play() FAILED:", err);
          setPlayingId(null);
          setCurrentTime(0);
        });
      }
    } catch (e) {
      console.error("[Play audio] Exception in handlePlay:", e);
      setPlayingId(null);
      setCurrentTime(0);
    }
  };

  const handleRepeatPaste = async (id: number) => {
    try {
      await invoke("repeat_paste_history_record", { id });
      setPastingId(id);
      setTimeout(() => {
        setPastingId(prev => (prev === id ? null : prev));
      }, 1500);
    } catch (e) {
      console.error(e);
    }
  };

  const formatTimestamp = (ts: string) => {
    const d = new Date(ts);
    return d.toLocaleString(lang === "ru" ? "ru-RU" : "en-US", {
      month: "short", day: "numeric", hour: "2-digit", minute: "2-digit"
    });
  };

  return (
    <div className="w-full max-w-3xl animate-in fade-in slide-in-from-bottom-4 duration-300">
      <div className="mb-8">
        <h2 className="text-3xl font-bold text-primary tracking-tight mb-2">
          {t(lang, "history.title")}
        </h2>
        <p className="text-secondary text-base max-w-2xl">
          {t(lang, "history.subtitle")}
        </p>
      </div>

      <audio
        ref={audioRef}
        style={{ display: "none" }}
        onTimeUpdate={() => {
          if (audioRef.current) setCurrentTime(audioRef.current.currentTime);
        }}
        onEnded={() => {
          setPlayingId(null);
          setCurrentTime(0);
        }}
        onError={(e) => {
          const target = e.currentTarget;
          console.error("[Play audio] <audio> onError fired!", e, "error code:", target.error?.code, "message:", target.error?.message);
          if (playingId !== null) {
            setPlayingId(null);
            setCurrentTime(0);
          }
        }}
      />

      {records.length === 0 ? (
        <div className="text-center text-secondary mt-20">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-12 h-12 mx-auto mb-4 opacity-50" strokeWidth="1.5">
            <circle cx="12" cy="12" r="10" />
            <polyline points="12 6 12 12 16 14" />
          </svg>
          <p className="text-lg font-medium text-primary">{t(lang, "history.empty")}</p>
          <p className="text-sm mt-1">{t(lang, "history.empty_desc")}</p>
        </div>
      ) : (
        <div className="flex flex-col gap-4">
          {records.map(r => (
            <div key={r.id} className="p-5 bg-surface border border-border rounded-2xl flex flex-col gap-3 group relative overflow-hidden">
              <div className="flex justify-between items-start gap-4">
                <div className="flex items-center gap-2">
                  {r.target_app_icon ? (
                    <img src={`data:image/png;base64,${r.target_app_icon}`} className="w-5 h-5 rounded" alt="" />
                  ) : (
                    <div className="w-5 h-5 bg-border rounded flex items-center justify-center">
                       <span className="text-[10px] text-secondary">?</span>
                    </div>
                  )}
                  <span className="text-sm font-medium text-secondary">{r.target_app_name}</span>
                  <span className="text-xs text-secondary/50">•</span>
                  <span className="text-xs text-secondary/80">{formatTimestamp(r.timestamp)}</span>
                  <span className="text-xs text-secondary/50">•</span>
                  <span className="text-xs text-secondary/80">{r.duration_sec.toFixed(1)}s</span>
                  <span className="text-xs text-secondary/50">•</span>
                  <span className="text-xs text-secondary/80">{models[r.model_id] || r.model_id}</span>
                </div>
                
                <div className="flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                  <button onClick={() => handleDelete(r.id)} className="p-1.5 text-secondary hover:text-[#ff453a] hover:bg-[#ff453a]/10 rounded-lg transition-colors" title={t(lang, "history.delete")}>
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4" strokeWidth="2"><path d="M3 6h18"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>
                  </button>
                </div>
              </div>

              <div className="text-base text-primary whitespace-pre-wrap leading-relaxed">
                {r.text}
              </div>

              {playingId === r.id && (
                <div className="w-full bg-border/60 rounded-full h-1 my-1 overflow-hidden">
                  <div
                    className="bg-accent h-full transition-all duration-100 ease-linear"
                    style={{ width: `${Math.min(100, (currentTime / (duration || r.duration_sec || 1)) * 100)}%` }}
                  />
                </div>
              )}

              <div className="flex flex-wrap gap-2 mt-1">
                <button
                  onClick={() => handlePlay(r.id)}
                  disabled={!r.has_audio}
                  title={!r.has_audio ? t(lang, "history.audio_unavailable") : undefined}
                  className={`px-3 py-1.5 bg-window border rounded-lg text-sm font-medium transition-colors flex items-center gap-1.5 ${
                    playingId === r.id
                      ? "border-accent text-primary"
                      : "border-border text-primary hover:border-secondary/40"
                  } disabled:opacity-50 disabled:cursor-not-allowed`}
                >
                  {playingId === r.id ? (
                    <>
                      <svg viewBox="0 0 24 24" fill="currentColor" className="w-4 h-4"><rect x="6" y="6" width="12" height="12" rx="1" /></svg>
                      {t(lang, "history.stop")}
                    </>
                  ) : (
                    <>
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4" strokeWidth="2" strokeLinejoin="round"><polygon points="5 3 19 12 5 21 5 3"/></svg>
                      {t(lang, "history.play")}
                    </>
                  )}
                </button>
                
                <button
                  onClick={() => handleCopy(r.id, r.text)}
                  title={t(lang, "history.copy_tooltip")}
                  className={`px-3 py-1.5 bg-window border rounded-lg text-sm font-medium transition-colors flex items-center gap-1.5 ${
                    copiedId === r.id
                      ? "border-success/60 text-success"
                      : "border-border text-primary hover:border-secondary/40"
                  }`}
                >
                  {copiedId === r.id ? (
                    <>
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4" strokeWidth="2" strokeLinejoin="round" strokeLinecap="round"><polyline points="20 6 9 17 4 12"/></svg>
                      {t(lang, "history.copied")}
                    </>
                  ) : (
                    <>
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4" strokeWidth="2" strokeLinejoin="round" strokeLinecap="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
                      {t(lang, "history.copy")}
                    </>
                  )}
                </button>
                
                <button
                  onClick={() => handleRepeatPaste(r.id)}
                  title={t(lang, "history.repeat_paste_tooltip")}
                  className={`px-3 py-1.5 bg-window border rounded-lg text-sm font-medium transition-colors flex items-center gap-1.5 ${
                    pastingId === r.id
                      ? "border-accent text-primary animate-pulse"
                      : "border-border text-primary hover:border-secondary/40"
                  }`}
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4" strokeWidth="2" strokeLinejoin="round" strokeLinecap="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="12" y1="18" x2="12" y2="12"/><line x1="9" y1="15" x2="15" y2="15"/></svg>
                  {pastingId === r.id ? t(lang, "history.repeat_paste_active") : t(lang, "history.repeat_paste")}
                </button>

                {r.has_audio && (
                  <button onClick={() => handleRetranscribe(r.id)} disabled={retranscribing[r.id]} className="px-3 py-1.5 bg-window border border-border rounded-lg text-sm font-medium text-primary hover:border-secondary/40 transition-colors flex items-center gap-1.5 disabled:opacity-50">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className={`w-4 h-4 ${retranscribing[r.id] ? 'animate-spin' : ''}`} strokeWidth="2" strokeLinejoin="round" strokeLinecap="round"><path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.92-10.44l5.58 5.58"/></svg>
                    {retranscribing[r.id] ? t(lang, "history.retranscribing") : t(lang, "history.retranscribe")}
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
