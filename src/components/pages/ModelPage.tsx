import { invoke } from "@tauri-apps/api/core";
import { Language, t } from "../../i18n";

export interface ModelInfo {
  id: string;
  name: string;
  size_mb: number;
  is_downloaded: boolean;
  is_active: boolean;
  accuracy_rating: number;
  speed_rating: number;
  languages: number;
  quantization: string;
  translation: TranslationSupport;
}

export type TranslationSupport = 
  | { type: "None" } 
  | { type: "ToEnglishOnly" } 
  | { type: "Pairs", langs: string[] };

interface ModelPageProps {
  lang: Language;
  models: ModelInfo[];
  downloading: Record<string, boolean>;
  progressMap: Record<string, number>;
  onDownload: (id: string) => void;
  onActivate: (id: string) => void;
}

const MODEL_TAGS: Record<string, string[]> = {
  "tiny": ["Punctuation", "Multilingual", "Для слабых ПК"],
  "base": ["Punctuation", "Multilingual"],
  "small": ["Punctuation", "Multilingual"],
  "medium": ["Punctuation", "Multilingual"],
  "large-v3-turbo": ["Punctuation", "Multilingual", "Максимальное качество"],
  "parakeet": ["Multilingual"],
  "canary": ["Multilingual"],
  "gigaam": ["Punctuation", "Russian"],
  "nemotron": ["Punctuation", "Multilingual"],
  "qwen": ["Punctuation", "Multilingual"]
};

const WaveformDivider = () => (
  <div className="w-full h-4 flex items-center justify-between opacity-30 my-4 px-2">
    {[...Array(60)].map((_, i) => {
      // Create a pseudo-random waveform pattern that is symmetrical or just interesting
      const height = Math.abs(Math.sin(i * 0.2)) * Math.abs(Math.sin(i * 0.05)) * 100;
      const hStr = height > 70 ? '12px' : height > 40 ? '8px' : height > 15 ? '4px' : '2px';
      return (
        <div key={i} className="w-[2px] bg-accent rounded-full" style={{ height: hStr }} />
      );
    })}
  </div>
);

const RatingDots = ({ rating }: { rating: number }) => (
  <div className="flex gap-1.5 mt-1.5">
    {[1, 2, 3, 4, 5].map((val) => (
      <div 
        key={val} 
        className={`w-2.5 h-2.5 rounded-full ${val <= rating ? 'bg-accent shadow-[var(--shadow-accent-xs)]' : 'bg-border'}`} 
      />
    ))}
  </div>
);

export function ModelPage({ lang, models, downloading, progressMap, onDownload, onActivate }: ModelPageProps) {
  const handleDelete = async (id: string) => {
    try {
      await invoke("delete_model", { id });
      window.location.reload();
    } catch (e) {
      console.error("Failed to delete model", e);
    }
  };

  return (
    <div className="w-full max-w-4xl animate-in fade-in slide-in-from-bottom-4 duration-300">
      <div className="mb-8">
        <h2 className="text-3xl font-bold text-primary tracking-tight mb-2">
          {t(lang, "model.title")}
        </h2>
        <p className="text-secondary text-base max-w-2xl">
          {t(lang, "model.subtitle")}
        </p>
      </div>

      {(() => {
        const activeModel = models.find(m => m.is_active);
        const recommendedIds = ["parakeet", "gigaam"];
        const remainingModels = models.filter(m => !m.is_active);
        
        const recommendedModels = remainingModels.filter(m => recommendedIds.includes(m.id));
        const otherModels = remainingModels.filter(m => !recommendedIds.includes(m.id));

        const renderCard = (m: ModelInfo) => {
          const isDownloading = downloading[m.id];
          const progress = progressMap[m.id] || 0;
          const isActive = m.is_active;
          
          return (
            <div 
              key={m.id} 
              className={`flex flex-col p-6 bg-surface border transition-all duration-300 relative overflow-hidden group ${
                isActive 
                  ? 'border-accent shadow-[var(--shadow-accent-lg)] rounded-[20px]' 
                  : 'border-border hover:border-border/80 rounded-[20px]'
              }`}
            >
              {/* Top Row: Name and Badges */}
              <div className="flex justify-between items-start mb-2">
                <h3 className="text-2xl font-bold text-primary tracking-tight">{m.name}</h3>
                
                <div className="flex flex-wrap justify-end items-center gap-2">
                  {isActive && (
                    <div className="px-3.5 py-1.5 rounded-full border border-accent/40 text-accent text-xs font-semibold flex items-center gap-2 mr-2">
                      <span className="w-1.5 h-1.5 rounded-full bg-accent shadow-[0_0_6px_currentColor]"></span>
                      {t(lang, "model.active")}
                    </div>
                  )}
                  {m.translation && m.translation.type !== "None" && (
                    <div className="relative group/tooltip px-3 py-1.5 rounded-full bg-border/60 text-secondary text-[11px] font-medium tracking-wide cursor-help">
                      {m.translation.type === "ToEnglishOnly" ? t(lang, "model.translation_en") : t(lang, "model.translation_pairs")}
                      <div className="absolute top-full right-0 mt-2 w-max max-w-[250px] bg-surface border border-border p-2 rounded-lg text-xs text-secondary opacity-0 invisible group-hover/tooltip:opacity-100 group-hover/tooltip:visible transition-all z-10 text-center whitespace-normal shadow-lg">
                        {m.translation.type === "ToEnglishOnly" ? t(lang, "model.translation_en_tooltip") : t(lang, "model.translation_pairs_tooltip")}
                      </div>
                    </div>
                  )}
                  {(MODEL_TAGS[m.id] || []).map(tag => (
                    <div key={tag} className="px-3 py-1.5 rounded-full bg-border/60 text-secondary text-[11px] font-medium tracking-wide">
                      {t(lang, `model.tags.${tag}`)}
                    </div>
                  ))}
                </div>
              </div>

              {/* Description */}
              <p className="text-sm text-secondary/80 max-w-[80%] leading-relaxed">
                {t(lang, `model.desc.${m.id}`)}
              </p>

              {/* Decorative Waveform Divider */}
              <WaveformDivider />

              {/* Bottom Meta Row */}
              <div className="flex items-end justify-between mt-1">
                
                {/* Left: Ratings */}
                <div className="flex items-center gap-8">
                  <div className="flex flex-col">
                    <span className="text-[11px] text-secondary/60 mb-0.5">{t(lang, "model.speed")}</span>
                    <RatingDots rating={m.speed_rating} />
                  </div>
                  <div className="flex flex-col">
                    <span className="text-[11px] text-secondary/60 mb-0.5">{t(lang, "model.accuracy")}</span>
                    <RatingDots rating={m.accuracy_rating} />
                  </div>
                </div>

                {/* Right: Meta & Actions */}
                <div className="flex items-center gap-5 text-xs font-medium text-secondary/70">
                  
                  {/* Meta Info */}
                  <div className="flex items-center gap-4">
                    <div className="flex items-center gap-1.5">
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4 opacity-70" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <circle cx="12" cy="12" r="10" />
                        <line x1="2" y1="12" x2="22" y2="12" />
                        <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
                      </svg>
                      {m.languages} {t(lang, "model.languages")}
                    </div>
                    
                    <div className="w-px h-4 bg-border"></div>
                    
                    <div>{m.size_mb} MB</div>
                  </div>

                  {/* Actions */}
                  <div className="flex items-center gap-3">
                    {!m.is_downloaded && !isDownloading && (
                      <button 
                        className="px-5 py-2 bg-window border border-border hover:border-accent/50 text-primary rounded-full transition-all flex items-center gap-2 cursor-pointer group/btn"
                        onClick={() => onDownload(m.id)}
                      >
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4 text-accent" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                          <polyline points="7 10 12 15 17 10" />
                          <line x1="12" x2="12" y1="15" y2="3" />
                        </svg>
                        {t(lang, "model.download")}
                      </button>
                    )}

                    {isDownloading && (
                      <div className="px-5 py-2 bg-window border border-border rounded-full flex items-center gap-3 w-36">
                        <span className="text-accent text-[11px] font-bold shrink-0">{progress.toFixed(0)}%</span>
                        <div className="w-full bg-border rounded-full h-1 overflow-hidden">
                          <div 
                            className="bg-accent h-full transition-all duration-300 ease-out" 
                            style={{ width: `${progress}%` }} 
                          />
                        </div>
                      </div>
                    )}

                    {m.is_downloaded && !isActive && (
                      <button 
                        className="px-5 py-2 border border-border hover:border-primary text-primary rounded-full transition-all flex items-center gap-1.5 cursor-pointer"
                        onClick={() => onActivate(m.id)}
                      >
                        {t(lang, "model.select")}
                      </button>
                    )}

                    {m.is_downloaded && isActive && (
                      <div className="px-5 py-2 bg-accent text-accent-text rounded-full font-bold flex items-center gap-2">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                          <polyline points="20 6 9 17 4 12" />
                        </svg>
                        {t(lang, "model.selected")}
                      </div>
                    )}

                    {m.is_downloaded && (
                      <button 
                        className="p-2 text-secondary/50 hover:text-red-400 transition-colors cursor-pointer ml-1"
                        onClick={() => handleDelete(m.id)}
                        title={t(lang, "model.delete")}
                      >
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                          <polyline points="3 6 5 6 21 6" />
                          <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                        </svg>
                      </button>
                    )}
                  </div>

                </div>
              </div>
            </div>
          );
        };

        return (
          <div className="flex flex-col gap-8">
            {activeModel && (
              <div className="flex flex-col gap-5">
                {renderCard(activeModel)}
              </div>
            )}
            
            {recommendedModels.length > 0 && (
              <div>
                <h3 className="text-xl font-bold text-primary mb-4">{t(lang, "model.recommended")}</h3>
                <div className="flex flex-col gap-5">
                  {recommendedModels.map(renderCard)}
                </div>
              </div>
            )}
            
            {otherModels.length > 0 && (
              <div>
                <h3 className="text-xl font-bold text-primary mb-4">{t(lang, "model.others")}</h3>
                <div className="flex flex-col gap-5">
                  {otherModels.map(renderCard)}
                </div>
              </div>
            )}
          </div>
        );
      })()}
    </div>
  );
}
