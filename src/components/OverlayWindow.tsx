// The capsule is displayed on top of arbitrary desktop windows; status colors (red/cyan/green) are part of the core product language.
import { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getLanguage, t, Language } from "../i18n";

type AppInfo = {
  title: string;
  icon_base64: string;
};

function parseColorToRgb(colorStr: string): [number, number, number] | null {
  colorStr = colorStr.trim();
  if (colorStr.startsWith("#")) {
    let hex = colorStr.slice(1);
    if (hex.length === 3) {
      hex = hex.split("").map(c => c + c).join("");
    }
    if (hex.length === 6) {
      const num = parseInt(hex, 16);
      return [(num >> 16) & 255, (num >> 8) & 255, num & 255];
    }
  }
  const rgbMatch = colorStr.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
  if (rgbMatch) {
    return [parseInt(rgbMatch[1], 10), parseInt(rgbMatch[2], 10), parseInt(rgbMatch[3], 10)];
  }
  return null;
}

function getComputedAccentRgb(): [number, number, number] {
  const style = getComputedStyle(document.documentElement);
  const rgbStr = style.getPropertyValue('--accent-rgb').trim();
  if (rgbStr) {
    const parts = rgbStr.split(',').map(s => parseInt(s.trim(), 10));
    if (parts.length === 3 && !parts.some(isNaN)) {
      return [parts[0], parts[1], parts[2]];
    }
  }
  const accentStr = style.getPropertyValue('--accent').trim();
  if (accentStr) {
    const parsed = parseColorToRgb(accentStr);
    if (parsed) return parsed;
  }
  return [255, 85, 51];
}

// 1. СПЕКТР (Spectrum - High-DPI Canvas с принудительным выравниванием по физическим пикселям)
function SpectrumVisualizer({ level, colorClass = "bg-accent", pulse = false, count = 46, maxHeight = 24 }: { level: number, colorClass?: string, pulse?: boolean, count?: number, maxHeight?: number }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const levelRef = useRef(level);
  const pulseRef = useRef(pulse);

  useEffect(() => { levelRef.current = level; }, [level]);
  useEffect(() => { pulseRef.current = pulse; }, [pulse]);

  useEffect(() => {
    let animationFrameId: number;
    const canvas = canvasRef.current;
    if (!canvas) return;

    let heights: number[] = [];
    let lastTime = performance.now();

    const render = (now: number) => {
      const dt = Math.min((now - lastTime) / 1000, 0.1);
      lastTime = now;
      const timeSec = now / 1000;

      const rect = canvas.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;
      const wCss = Math.max(10, Math.floor(rect.width));
      const hCss = Math.max(10, Math.floor(rect.height));

      const wPhys = Math.round(wCss * dpr);
      const hPhys = Math.round(hCss * dpr);

      if (canvas.width !== wPhys || canvas.height !== hPhys) {
        canvas.width = wPhys;
        canvas.height = hPhys;
      }

      const ctx = canvas.getContext('2d');
      if (ctx) {
        // Reset transform to work in 100% integer physical pixels
        ctx.setTransform(1, 0, 0, 1, 0, 0);
        ctx.clearRect(0, 0, wPhys, hPhys);

        const isPulse = pulseRef.current;
        const [r, g, b] = isPulse || colorClass === "bg-processing" ? [77, 216, 230] : getComputedAccentRgb();
        const scaledLevel = isPulse ? 0.35 : Math.min(Math.pow(levelRef.current, 0.4) * 2.5, 1.0);

        // Strict uniform integer physical pixel metrics:
        const barWidthPx = Math.max(2, Math.round(2.0 * dpr));
        const gapPx = Math.max(2, Math.round(4.0 * dpr));
        const pitchPx = barWidthPx + gapPx;

        // Number of bars
        const totalWPx = count * barWidthPx + (count - 1) * gapPx;
        const startXPx = Math.floor((wPhys - totalWPx) / 2);
        const centerYPx = Math.floor(hPhys / 2);

        if (heights.length !== count) {
          heights = new Array(count).fill(3);
        }

        const maxHPx = Math.round((maxHeight || 24) * dpr);
        const minHPx = Math.max(3, Math.round(3 * dpr));
        const radiusPx = Math.min(barWidthPx / 2, Math.round(1.5 * dpr));

        ctx.fillStyle = `rgb(${r}, ${g}, ${b})`;

        for (let i = 0; i < count; i++) {
          const noise1 = Math.sin(timeSec * 4.5 + i * 0.73);
          const noise2 = Math.cos(timeSec * 2.8 - i * 0.41);
          const variation = 0.5 + 0.5 * ((noise1 + noise2) * 0.5);

          const livingBase = isPulse ? 0 : 0.04 + 0.05 * Math.sin(timeSec * 3.0 + i * 1.3);
          const activeLevel = isPulse ? 0.35 + 0.15 * variation : Math.max(livingBase, scaledLevel * (0.55 + 0.45 * variation));
          const targetHeight = minHPx + (maxHPx - minHPx) * activeLevel;

          const current = heights[i];
          const speed = targetHeight > current ? 18.0 : 5.0;
          heights[i] = current + (targetHeight - current) * Math.min(1.0, speed * dt);

          const hBarPx = Math.max(minHPx, Math.round(heights[i]));
          const xPx = startXPx + i * pitchPx;
          const yPx = Math.floor(centerYPx - hBarPx / 2);

          ctx.beginPath();
          ctx.roundRect(xPx, yPx, barWidthPx, hBarPx, radiusPx);
          ctx.fill();
        }
      }

      animationFrameId = requestAnimationFrame(render);
    };

    animationFrameId = requestAnimationFrame(render);
    return () => cancelAnimationFrame(animationFrameId);
  }, [colorClass, count, maxHeight]);

  return <canvas ref={canvasRef} className={`w-full h-full block ${pulse ? 'animate-pulse opacity-80' : ''}`} />;
}

// 2. СПЕКТР-ВОЛНА (Spectrum Wave - без разрывов)
function SpectrumWaveVisualizer({ level, colorClass = "bg-accent", pulse = false, maxHeight = 24 }: { level: number, colorClass?: string, pulse?: boolean, maxHeight?: number }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const levelRef = useRef(level);
  const pulseRef = useRef(pulse);

  useEffect(() => { levelRef.current = level; }, [level]);
  useEffect(() => { pulseRef.current = pulse; }, [pulse]);

  useEffect(() => {
    let animationFrameId: number;
    const canvas = canvasRef.current;
    if (!canvas) return;

    let heights: number[] = [];
    let phase = 0;
    let lastTime = performance.now();

    const render = (now: number) => {
      const dt = Math.min((now - lastTime) / 1000, 0.1);
      lastTime = now;

      const rect = canvas.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;
      const wCss = Math.max(10, Math.floor(rect.width));
      const hCss = Math.max(10, Math.floor(rect.height));

      const wPhys = Math.round(wCss * dpr);
      const hPhys = Math.round(hCss * dpr);

      if (canvas.width !== wPhys || canvas.height !== hPhys) {
        canvas.width = wPhys;
        canvas.height = hPhys;
      }

      const ctx = canvas.getContext('2d');
      if (ctx) {
        ctx.setTransform(1, 0, 0, 1, 0, 0);
        ctx.clearRect(0, 0, wPhys, hPhys);

        const isPulse = pulseRef.current;
        const [r, g, b] = isPulse || colorClass === "bg-processing" ? [77, 216, 230] : getComputedAccentRgb();
        const rawLevel = levelRef.current;
        const scaledLevel = isPulse ? 0.35 : Math.min(Math.pow(rawLevel, 0.37) * 2.55, 1.0);

        // Voice accelerates the wave travel speed dynamically:
        // Slow calm drift in silence (2.8 rad/s), surging energetically on speech (up to 7.5 rad/s)
        const waveSpeed = 2.8 + scaledLevel * 4.8;
        phase += waveSpeed * dt;

        const barWidthPx = Math.max(1, Math.round(1.5 * dpr));
        const gapPx = Math.max(2, Math.round(2.5 * dpr));
        const pitchPx = barWidthPx + gapPx;
        const count = Math.max(5, Math.floor((wPhys - Math.round(4 * dpr)) / pitchPx));
        const totalWPx = count * barWidthPx + (count - 1) * gapPx;
        const startXPx = Math.floor((wPhys - totalWPx) / 2);
        const centerYPx = Math.floor(hPhys / 2);

        if (heights.length !== count) {
          heights = new Array(count).fill(3 * dpr);
        }

        const maxHPx = Math.round((maxHeight || 24) * dpr);
        const minHPx = Math.max(2, Math.round(2.5 * dpr));
        const radiusPx = Math.min(barWidthPx / 2, 1.5 * dpr);

        ctx.fillStyle = `rgb(${r}, ${g}, ${b})`;

        for (let i = 0; i < count; i++) {
          const xPx = startXPx + i * pitchPx;
          const normX = i / count;

          // 1. Multi-harmonic wave components traveling with non-repeating interferences
          const w1 = Math.sin(normX * Math.PI * 2.8 - phase);
          const w2 = Math.sin(normX * Math.PI * 5.2 - phase * 1.55 + 1.2);
          const w3 = Math.cos(normX * Math.PI * 1.6 - phase * 0.75);
          const waveShape = (w1 * 0.50 + w2 * 0.30 + w3 * 0.20 + 1.0) * 0.5; // 0.0 .. 1.0
          const flowEnvelope = 0.15 + 0.85 * waveShape;

          // 2. Per-bar chaotic spectral turbulence (unique dancing energy for every bar)
          const j1 = Math.sin(phase * 2.3 + i * 1.57);
          const j2 = Math.cos(phase * 1.5 - i * 2.31);
          const j3 = Math.sin(phase * 3.7 + i * 3.19);
          const barTexture = 0.35 + 0.65 * (0.5 + 0.28 * j1 + 0.14 * j2 + 0.08 * j3);

          // 3. Resting wave in silence + rich dynamic crests & valleys on voice
          const idleBaseline = minHPx + Math.round(1.0 * dpr) * (0.5 + 0.5 * Math.sin(normX * Math.PI * 2.0 - phase * 0.6));
          const voiceModulation = scaledLevel * flowEnvelope * barTexture;
          const targetHeight = idleBaseline + (maxHPx - idleBaseline) * Math.min(1.0, voiceModulation * 1.25);

          const speed = targetHeight > heights[i] ? 22.0 : 6.5;
          heights[i] += (targetHeight - heights[i]) * Math.min(1.0, speed * dt);

          const barHPx = Math.max(minHPx, Math.round(heights[i]));
          const yPx = Math.floor(centerYPx - barHPx / 2);

          ctx.beginPath();
          ctx.roundRect(xPx, yPx, barWidthPx, barHPx, radiusPx);
          ctx.fill();
        }
      }

      animationFrameId = requestAnimationFrame(render);
    };

    animationFrameId = requestAnimationFrame(render);
    return () => cancelAnimationFrame(animationFrameId);
  }, [colorClass, maxHeight]);

  return <canvas ref={canvasRef} className={`w-full h-full block ${pulse ? 'animate-pulse opacity-80' : ''}`} />;
}

// 3. НИТИ-НЕОН (Neon Threads)
function NeonThreadsVisualizer({ level, colorClass = "bg-accent", pulse = false, maxHeight = 24 }: { level: number, colorClass?: string, pulse?: boolean, maxHeight?: number }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const levelRef = useRef(level);
  const pulseRef = useRef(pulse);

  useEffect(() => { levelRef.current = level; }, [level]);
  useEffect(() => { pulseRef.current = pulse; }, [pulse]);

  useEffect(() => {
    let animationFrameId: number;
    const canvas = canvasRef.current;
    if (!canvas) return;

    const render = (now: number) => {
      const timeSec = now / 1000;

      const rect = canvas.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;
      const w = Math.max(10, Math.floor(rect.width));
      const h = Math.max(10, Math.floor(rect.height));

      if (canvas.width !== w * dpr || canvas.height !== h * dpr) {
        canvas.width = w * dpr;
        canvas.height = h * dpr;
      }

      const ctx = canvas.getContext('2d');
      if (ctx) {
        ctx.setTransform(1, 0, 0, 1, 0, 0);
        ctx.scale(dpr, dpr);
        ctx.clearRect(0, 0, w, h);

        const isPulse = pulseRef.current;
        const [r, g, b] = isPulse || colorClass === "bg-processing" ? [77, 216, 230] : getComputedAccentRgb();
        const scaledLevel = isPulse ? 0.35 : Math.min(Math.pow(levelRef.current, 0.30) * 2.8, 1.0);

        const centerY = h / 2;
        const maxAmp = (maxHeight || 24) * 0.52;
        const amp = 3 + (maxAmp - 3) * (0.10 + scaledLevel * 0.90);

        const points = 36;
        const dx = w / (points - 1);

        // Thread 2 (soft secondary ribbon)
        ctx.beginPath();
        for (let i = 0; i < points; i++) {
          const x = i * dx;
          const normX = i / (points - 1);
          const y = centerY + Math.sin(normX * Math.PI * 2.8 + timeSec * 3.2) * (amp * 0.8) * Math.sin(normX * Math.PI);
          if (i === 0) ctx.moveTo(x, y);
          else ctx.lineTo(x, y);
        }
        ctx.strokeStyle = `rgba(${r}, ${g}, ${b}, 0.38)`;
        ctx.lineWidth = 1.5;
        ctx.lineCap = 'round';
        ctx.stroke();

        // Thread 1 (bright primary ribbon)
        ctx.beginPath();
        for (let i = 0; i < points; i++) {
          const x = i * dx;
          const normX = i / (points - 1);
          const y = centerY + Math.sin(normX * Math.PI * 2.2 - timeSec * 4.0 + 1.2) * amp * Math.sin(normX * Math.PI);
          if (i === 0) ctx.moveTo(x, y);
          else ctx.lineTo(x, y);
        }
        ctx.strokeStyle = `rgb(${r}, ${g}, ${b})`;
        ctx.lineWidth = 2.0;
        ctx.lineCap = 'round';
        ctx.stroke();

        // Head comet particle
        const headX = (timeSec * 50) % w;
        const headNorm = headX / w;
        const headY = centerY + Math.sin(headNorm * Math.PI * 2.2 - timeSec * 4.0 + 1.2) * amp * Math.sin(headNorm * Math.PI);
        ctx.fillStyle = `rgb(${r}, ${g}, ${b})`;
        ctx.beginPath();
        ctx.arc(headX, headY, 2.0, 0, Math.PI * 2);
        ctx.fill();
      }

      animationFrameId = requestAnimationFrame(render);
    };

    animationFrameId = requestAnimationFrame(render);
    return () => cancelAnimationFrame(animationFrameId);
  }, [colorClass, maxHeight]);

  return <canvas ref={canvasRef} className={`w-full h-full block ${pulse ? 'animate-pulse opacity-80' : ''}`} />;
}

// 4. ЧАСТИЦЫ (Soft Particles)
class SoftParticle {
  w: number;
  h: number;
  x: number = 0;
  y: number = 0;
  vx: number = 0;
  vy: number = 0;
  baseSize: number = 1.2;
  baseAlpha: number = 0.4;
  seed: number;
  angle: number;

  constructor(w: number, h: number) {
    this.w = w;
    this.h = h;
    this.seed = Math.random() * 100;
    this.angle = Math.random() * Math.PI * 2;
    this.reset();
  }
  reset() {
    this.x = Math.random() * this.w;
    this.y = Math.random() * this.h;
    this.vx = (Math.random() - 0.5) * 0.2;
    this.vy = (Math.random() - 0.5) * 0.2;
    this.baseSize = 1.0 + Math.random() * 1.2;
    this.baseAlpha = 0.3 + Math.random() * 0.45;
  }
  update(level: number, dt: number, timeSec: number) {
    const driftX = Math.sin(this.seed + timeSec * 1.2) * 0.18;
    const driftY = Math.cos(this.seed + timeSec * 0.9) * 0.18;

    const swell = level * 0.45;
    this.vx += (driftX + Math.cos(this.angle) * swell - this.vx * 0.08) * dt * 30;
    this.vy += (driftY + Math.sin(this.angle) * swell - this.vy * 0.08) * dt * 30;

    const speed = Math.hypot(this.vx, this.vy);
    const maxSpeed = 0.8 + level * 1.8;
    if (speed > maxSpeed) {
      this.vx = (this.vx / speed) * maxSpeed;
      this.vy = (this.vy / speed) * maxSpeed;
    }

    this.x += this.vx * dt * 50;
    this.y += this.vy * dt * 50;

    if (this.x < 0) this.x = this.w;
    if (this.x > this.w) this.x = 0;
    if (this.y < 0) this.y = this.h;
    if (this.y > this.h) this.y = 0;
  }
  draw(ctx: CanvasRenderingContext2D, r: number, g: number, b: number, level: number) {
    const currentSize = this.baseSize * (1.0 + level * 0.5);
    const alpha = Math.min(0.95, this.baseAlpha + level * 0.35);
    ctx.fillStyle = `rgba(${r}, ${g}, ${b}, ${alpha})`;
    ctx.beginPath();
    ctx.arc(this.x, this.y, currentSize, 0, Math.PI * 2);
    ctx.fill();
  }
}

function ParticlesVisualizer({ level, colorClass = "bg-accent", pulse = false, count = 32 }: { level: number, colorClass?: string, pulse?: boolean, count?: number }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const levelRef = useRef(level);
  const pulseRef = useRef(pulse);

  useEffect(() => { levelRef.current = level; }, [level]);
  useEffect(() => { pulseRef.current = pulse; }, [pulse]);

  useEffect(() => {
    let animationFrameId: number;
    const canvas = canvasRef.current;
    if (!canvas) return;

    let particles: SoftParticle[] = [];
    let lastTime = performance.now();

    const render = (now: number) => {
      const dt = Math.min((now - lastTime) / 1000, 0.1);
      lastTime = now;
      const timeSec = now / 1000;

      const rect = canvas.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;
      const w = Math.max(10, Math.floor(rect.width));
      const h = Math.max(10, Math.floor(rect.height));

      if (canvas.width !== w * dpr || canvas.height !== h * dpr) {
        canvas.width = w * dpr;
        canvas.height = h * dpr;
      }

      const ctx = canvas.getContext('2d');
      if (ctx) {
        ctx.setTransform(1, 0, 0, 1, 0, 0);
        ctx.scale(dpr, dpr);
        ctx.clearRect(0, 0, w, h);

        const isPulse = pulseRef.current;
        const [r, g, b] = isPulse || colorClass === "bg-processing" ? [77, 216, 230] : getComputedAccentRgb();
        const scaledLevel = isPulse ? 0.35 : Math.min(Math.pow(levelRef.current, 0.4) * 2.0, 1.0);

        if (particles.length !== count) {
          particles = Array.from({ length: count }, () => new SoftParticle(w, h));
        }

        for (const p of particles) {
          p.w = w;
          p.h = h;
          p.update(scaledLevel, dt, timeSec);
          p.draw(ctx, r, g, b, scaledLevel);
        }
      }

      animationFrameId = requestAnimationFrame(render);
    };

    animationFrameId = requestAnimationFrame(render);
    return () => cancelAnimationFrame(animationFrameId);
  }, [colorClass, count]);

  return <canvas ref={canvasRef} className={`w-full h-full block ${pulse ? 'animate-pulse opacity-80' : ''}`} />;
}

// Unified EqualizerVisualizer
function EqualizerVisualizer(props: {
  style?: string;
  level: number;
  colorClass?: string;
  pulse?: boolean;
  count?: number;
  maxHeight?: number;
}) {
  const { style = "spectrum", ...rest } = props;
  if (style === "spectrum_wave") {
    return <SpectrumWaveVisualizer {...rest} />;
  }
  if (style === "neon_threads") {
    return <NeonThreadsVisualizer {...rest} />;
  }
  if (style === "particles") {
    return <ParticlesVisualizer {...rest} count={props.count ? Math.min(props.count, 42) : 32} />;
  }
  return <SpectrumVisualizer {...rest} />;
}

function useOverlayState() {
  const [level, setLevel] = useState(0);
  const [statusText, setStatusText] = useState("");
  const [liveText, setLiveText] = useState<string | null>(null);
  const [errorText, setErrorText] = useState<string | null>(null);
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);

  const [isRecording, setIsRecording] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);
  const [isSuccess, setIsSuccess] = useState(false);
  const [isCopied, setIsCopied] = useState(false);
  const [isError, setIsError] = useState(false);
  
  const [lang, setLang] = useState<Language>("en");
  const [skin, setSkin] = useState<"full" | "compact" | "mini" | string>("full");
  const [equalizerStyle, setEqualizerStyle] = useState<string>("spectrum");

  const hideTimeoutRef = useRef<number | null>(null);

  const clearHideTimeout = () => {
    if (hideTimeoutRef.current !== null) {
      window.clearTimeout(hideTimeoutRef.current);
      hideTimeoutRef.current = null;
    }
  };

  useEffect(() => {
    invoke<any>("get_settings").then(settings => {
      setLang(getLanguage(settings.app_language));
      if (settings.overlay_skin) {
        setSkin(settings.overlay_skin);
      }
      if (settings.equalizer_style) {
        setEqualizerStyle(settings.equalizer_style);
      }
    });

    const unlistenLevel = listen<number>("audio-level", (event) => {
      setLevel(event.payload);
    });

    const unlistenLive = listen<string>("live-transcription", (event) => {
      setLiveText(event.payload);
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
      clearHideTimeout();
      setStatusText(t(lang, "overlay.recording")); // In compact, text is "Listening", we can override in component
      setLiveText(null);
      setIsRecording(true);
      setIsProcessing(false);
      setIsSuccess(false);
      setIsError(false);
      setErrorText(null);
    });

    const unlistenProcessing = listen("processing-started", () => {
      setStatusText(t(lang, "overlay.processing"));
      setLiveText(null);
      setIsRecording(false);
      setIsProcessing(true);
      setIsSuccess(false);
      setIsError(false);
    });

    const unlistenDone = listen<string>("transcription-done", (event) => {
      setLiveText(null);
      const payload = event.payload;
      if (payload.startsWith("Error: ")) {
        const code = payload.replace("Error: ", "");
        // "err_speech_not_recognized" usually just shows silently without red error if user canceled
        if (code === "err_speech_not_recognized") {
          setIsRecording(false);
          setIsProcessing(false);
          setIsSuccess(false);
          setIsCopied(false);
          setIsError(false);
          setErrorText(null);
          clearHideTimeout();
          return;
        }
        
        setErrorText(t(lang, `errors.${code}`));
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
      
      clearHideTimeout();
      const hideDuration = payload.length > 80 ? 3000 : 1500;
      hideTimeoutRef.current = window.setTimeout(() => {
        setIsSuccess(false);
        setIsCopied(false);
        setIsError(false);
      }, hideDuration);
    });

    const unlistenError = listen<string>("show-error", (event) => {
      // event.payload is now an error code, e.g. "err_mic_not_found"
      setLiveText(null);
      setErrorText(t(getLanguage(lang), `errors.${event.payload}`));
      setIsRecording(false);
      setIsProcessing(false);
      setIsSuccess(false);
      setIsError(true);
      
      clearHideTimeout();
      hideTimeoutRef.current = window.setTimeout(() => {
        setIsError(false);
        setErrorText(null);
      }, 2000);
    });

    const unlistenCancelled = listen("recording-cancelled", () => {
      clearHideTimeout();
      setLiveText(null);
      setIsRecording(false);
      setIsProcessing(false);
      setIsSuccess(false);
      setIsError(false);
    });

    const unlistenCancelledSilently = listen("recording-cancelled-silently", () => {
      clearHideTimeout();
      setLiveText(null);
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
         if (settings.equalizer_style) {
           setEqualizerStyle(settings.equalizer_style);
         }
       });
    });

    return () => {
      unlistenLevel.then(f => f());
      unlistenLive.then(f => f());
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

  return { level, statusText, liveText, errorText, appInfo, isRecording, isProcessing, isSuccess, isCopied, isError, lang, skin, equalizerStyle };
}

function OverlayFull(props: ReturnType<typeof useOverlayState>) {
  const { level, statusText, liveText, errorText, appInfo, isRecording, isProcessing, isSuccess, isError, isCopied, lang } = props;
  const contentRef = useRef<HTMLDivElement>(null);
  const [cardHeight, setCardHeight] = useState(103);
  const [isExpanding, setIsExpanding] = useState(false);

  let glowClass = "shadow-lg";
  let footerText = t(lang, "overlay.cancel");
  if (isRecording) {
    glowClass = "animate-glow-pulse";
  } else if (isProcessing) {
    glowClass = "shadow-[0_0_20px_rgba(77,216,230,0.34)]";
    footerText = t(lang, "overlay.processing");
  } else if (isSuccess) {
    glowClass = "shadow-[0_0_20px_rgba(126,212,145,0.33)]";
  } else if (isError) {
    glowClass = "shadow-[0_0_20px_rgba(255,85,51,0.2)]";
  }

  const isActive = isRecording || isProcessing || isSuccess || isError;
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    if (isActive) {
      const t = setTimeout(() => setIsVisible(true), 10);
      return () => clearTimeout(t);
    } else {
      setIsVisible(false);
    }
  }, [isActive]);

  const hasLiveText = isRecording && !!liveText;
  const hasFinalText = isSuccess && !isCopied && !!statusText;

  useEffect(() => {
    if ((hasLiveText || hasFinalText) && contentRef.current) {
      const textScrollHeight = contentRef.current.scrollHeight;
      const headerFooterHeight = 55;
      const neededCardH = Math.max(103, headerFooterHeight + textScrollHeight + 12);

      const maxScreenH = window.screen.availHeight ? window.screen.availHeight * 0.75 : 600;
      const maxCardH = Math.floor(maxScreenH - 48);

      const targetCardH = Math.min(maxCardH, neededCardH);
      const targetWindowH = targetCardH + 48;

      if (targetCardH > 103) {
        setIsExpanding(true);
        const timer = setTimeout(() => {
          setIsExpanding(false);
        }, 320);

        setCardHeight(targetCardH);
        invoke("resize_overlay_window", { logicalHeight: targetWindowH }).catch(() => {});

        return () => clearTimeout(timer);
      } else {
        setIsExpanding(false);
        setCardHeight(103);
        invoke("resize_overlay_window", { logicalHeight: 151 }).catch(() => {});
      }
    } else {
      setIsExpanding(false);
      setCardHeight(103);
      invoke("resize_overlay_window", { logicalHeight: 151 }).catch(() => {});
    }
  }, [hasLiveText, hasFinalText, statusText, liveText, isSuccess, isCopied, isRecording, isProcessing]);

  return (
    <div className="flex w-full h-full items-end justify-center pb-6 px-6 pt-6 bg-transparent">
      <div 
        style={{ height: `${cardHeight}px` }}
        className={`w-[311px] bg-overlay/95 backdrop-blur-md rounded-[20px] flex flex-col transition-[height,box-shadow,opacity,transform] duration-300 ${glowClass} ${isVisible ? 'opacity-100 scale-100' : 'opacity-0 scale-95'} overflow-hidden`}
      >
        {/* Header */}
        <div className="flex flex-row items-center justify-between w-full px-3 py-1.5 border-b border-border/50 bg-surface/50 shrink-0 h-[32px]">
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
        <div className="flex-1 w-full flex items-center justify-center px-4 py-1.5 overflow-hidden relative min-h-0">
          {hasLiveText ? (
            <div 
              ref={contentRef} 
              className={`w-full max-h-full ${isExpanding ? 'overflow-hidden' : 'overflow-y-auto'} px-1 py-1 text-center [&::-webkit-scrollbar]:w-1 [&::-webkit-scrollbar-thumb]:bg-secondary/40 [&::-webkit-scrollbar-thumb]:rounded-full`}
            >
              <span className="text-primary text-xs tracking-wide text-center leading-relaxed block break-words">
                {liveText}
              </span>
            </div>
          ) : (
            <>
              {isRecording && <EqualizerVisualizer style={props.equalizerStyle} level={level} colorClass="bg-accent" />}
              {isProcessing && <EqualizerVisualizer style={props.equalizerStyle} level={0.0} colorClass="bg-processing" pulse />}
              {isSuccess && (
                <div 
                  ref={contentRef} 
                  className={`w-full max-h-full ${isExpanding ? 'overflow-hidden' : 'overflow-y-auto'} px-1 py-1 text-center [&::-webkit-scrollbar]:w-1 [&::-webkit-scrollbar-thumb]:bg-secondary/40 [&::-webkit-scrollbar-thumb]:rounded-full`}
                >
                  <span className="text-primary text-xs tracking-wide text-center leading-relaxed block break-words">
                    {statusText}
                  </span>
                </div>
              )}
              {isError && (
                <span className="text-error text-xs font-medium text-center">
                  {errorText || t(lang, "overlay.error_no_mic")}
                </span>
              )}
            </>
          )}
        </div>

        {/* Footer */}
        <div className="w-full text-center py-1 border-t border-border/50 bg-surface/50 shrink-0 h-[23px] flex items-center justify-center">
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
  const [dotCount, setDotCount] = useState(23);

  useEffect(() => {
    if (!containerRef.current) return;
    const observer = new ResizeObserver(entries => {
      for (let entry of entries) {
        const width = entry.contentRect.width;
        // Bar is 2.0px + gap is 4.0px. Total pitch = 6.0px.
        // Total width W = N * 2 + (N - 1) * 4 = 6N - 4
        // N = Math.floor((W + 4) / 6)
        setDotCount(Math.max(1, Math.floor((width + 4) / 6)));
      }
    });
    observer.observe(containerRef.current);
    return () => observer.disconnect();
  }, []);

  let glowClass = "shadow-lg";

  if (isRecording) {
    glowClass = "animate-glow-pulse";
  } else if (isProcessing) {
    glowClass = "shadow-[0_0_20px_rgba(77,216,230,0.34)]";
  } else if (isSuccess) {
    glowClass = "shadow-[0_0_20px_rgba(126,212,145,0.33)] animate-out slide-out-to-bottom-4 duration-500 delay-500";
  } else if (isError) {
    glowClass = "shadow-[0_0_20px_rgba(255,85,51,0.2)]";
  }

  const isActive = isRecording || isProcessing || isSuccess || isError;
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    if (isActive) {
      const t = setTimeout(() => setIsVisible(true), 10);
      return () => clearTimeout(t);
    } else {
      setIsVisible(false);
    }
  }, [isActive]);

  return (
    <div className="flex w-full h-full items-center justify-center p-6 bg-transparent">
      <div className={`w-[199px] h-[44px] px-4 bg-overlay/95 backdrop-blur-md rounded-full flex flex-row items-center transition-all duration-300 ${glowClass} ${isVisible ? 'opacity-100 scale-100' : 'opacity-0 scale-95'} overflow-hidden relative`}>
        
        {/* RECORDING STATE */}
        <div className={`absolute inset-0 px-4 flex flex-row items-center transition-opacity duration-200 ${isRecording ? "opacity-100" : "opacity-0 pointer-events-none"}`}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4 text-accent shrink-0" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
            <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"></path>
            <path d="M19 10v2a7 7 0 0 1-14 0v-2"></path>
            <line x1="12" x2="12" y1="19" y2="22"></line>
          </svg>
          <div ref={containerRef} className="flex-1 flex justify-center items-center overflow-hidden pl-4 pr-0">
            <EqualizerVisualizer style={props.equalizerStyle} level={level} colorClass="bg-accent" count={dotCount} />
          </div>
        </div>

        {/* PROCESSING STATE */}
        <div className={`absolute inset-0 px-4 flex flex-row items-center justify-center transition-opacity duration-200 ${isProcessing ? "opacity-100" : "opacity-0 pointer-events-none"}`}>
           <div className="flex flex-row items-center gap-2 mr-2">
             <div className="w-1.5 h-1.5 rounded-full bg-processing animate-processing-dot" style={{ animationDelay: '0ms' }}></div>
             <div className="w-1.5 h-1.5 rounded-full bg-processing animate-processing-dot" style={{ animationDelay: '150ms' }}></div>
             <div className="w-1.5 h-1.5 rounded-full bg-processing animate-processing-dot" style={{ animationDelay: '300ms' }}></div>
           </div>
           <span className="text-secondary text-[10px] font-medium truncate">{t(lang, "overlay.processing")}</span>
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
           <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-5 h-5 text-error shrink-0" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
             <line x1="18" y1="6" x2="6" y2="18"></line>
             <line x1="6" y1="6" x2="18" y2="18"></line>
           </svg>
           <span className="text-error text-xs font-medium truncate flex-1 ml-2 text-center">
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
  const [dotCount, setDotCount] = useState(11);

  useEffect(() => {
    if (!containerRef.current) return;
    const observer = new ResizeObserver(entries => {
      for (let entry of entries) {
        const width = entry.contentRect.width;
        // Bar is 2.0px + gap is 4.0px. Total pitch = 6.0px.
        // Total width W = N * 2 + (N - 1) * 4 = 6N - 4
        // N = Math.floor((W + 4) / 6)
        setDotCount(Math.max(1, Math.floor((width + 4) / 6)));
      }
    });
    observer.observe(containerRef.current);
    return () => observer.disconnect();
  }, []);

  let glowClass = "shadow-lg";

  if (isRecording) {
    glowClass = "animate-glow-pulse";
  } else if (isProcessing) {
    glowClass = "shadow-[0_0_20px_rgba(77,216,230,0.34)]";
  } else if (isSuccess) {
    glowClass = "shadow-[0_0_20px_rgba(126,212,145,0.33)] animate-out slide-out-to-bottom-4 duration-500 delay-500";
  } else if (isError) {
    glowClass = "shadow-[0_0_20px_rgba(255,85,51,0.2)]";
  }

  const isActive = isRecording || isProcessing || isSuccess || isError;
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    if (isActive) {
      const t = setTimeout(() => setIsVisible(true), 10);
      return () => clearTimeout(t);
    } else {
      setIsVisible(false);
    }
  }, [isActive]);

  return (
    <div className="flex w-full h-full items-center justify-center p-6 bg-transparent">
      <div className={`w-[86px] h-[34px] px-2.5 bg-overlay/95 backdrop-blur-md rounded-full flex flex-row items-center justify-center transition-all duration-300 ${glowClass} ${isVisible ? 'opacity-100 scale-100' : 'opacity-0 scale-95'} overflow-hidden relative`}>
        
        {/* RECORDING STATE */}
        <div className={`absolute inset-0 px-2.5 flex flex-row items-center justify-center transition-opacity duration-200 ${isRecording ? "opacity-100" : "opacity-0 pointer-events-none"}`}>
          <div ref={containerRef} className="w-full h-6 flex justify-center items-center overflow-hidden">
            <EqualizerVisualizer style={props.equalizerStyle} level={level} colorClass="bg-accent" count={dotCount} maxHeight={22} />
          </div>
        </div>

        {/* PROCESSING STATE */}
        <div className={`absolute inset-0 flex flex-row items-center justify-center transition-opacity duration-200 ${isProcessing ? "opacity-100" : "opacity-0 pointer-events-none"}`}>
           <div className="flex flex-row items-center gap-2.5">
             <div className="w-1.5 h-1.5 rounded-full bg-processing animate-processing-dot" style={{ animationDelay: '0ms' }}></div>
             <div className="w-1.5 h-1.5 rounded-full bg-processing animate-processing-dot" style={{ animationDelay: '150ms' }}></div>
             <div className="w-1.5 h-1.5 rounded-full bg-processing animate-processing-dot" style={{ animationDelay: '300ms' }}></div>
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
           <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-4 h-4 text-error" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
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
  const [isRendered, setIsRendered] = useState(false);

  const isActive = state.isRecording || state.isProcessing || state.isSuccess || state.isError;

  useEffect(() => {
    if (isActive) {
      setIsRendered(true);
    } else {
      const t = setTimeout(() => {
        setIsRendered(false);
        invoke("hide_overlay");
      }, 300);
      return () => clearTimeout(t);
    }
  }, [isActive]);

  if (!isRendered && !isActive) {
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
