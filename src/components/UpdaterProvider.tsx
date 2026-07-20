import React, { createContext, useContext, useEffect, useState, useRef } from "react";
import { check, Update } from "@tauri-apps/plugin-updater";
import { getVersion } from "@tauri-apps/api/app";

export type UpdateStatus = 
  | "available"
  | "up-to-date"
  | "no-internet"
  | "server-unavailable"
  | "invalid-signature"
  | "error";

export interface UpdateCheckResult {
  status: UpdateStatus;
  reason?: string;
}

interface UpdaterContextType {
  update: Update | null;
  currentVersion: string;
  isChecking: boolean;
  checkForUpdates: () => Promise<UpdateCheckResult | null>;
  clearUpdate: () => void;
}

const UpdaterContext = createContext<UpdaterContextType | null>(null);

export function UpdaterProvider({ children }: { children: React.ReactNode }) {
  const [update, setUpdate] = useState<Update | null>(null);
  const [currentVersion, setCurrentVersion] = useState("0.1.0");
  const [isChecking, setIsChecking] = useState(false);
  const hasCheckedSilently = useRef(false);

  useEffect(() => {
    getVersion().then(setCurrentVersion).catch(console.error);
  }, []);

  useEffect(() => {
    if (hasCheckedSilently.current) return;
    hasCheckedSilently.current = true;

    // Silent check on startup (wait 3s to not block UI)
    const timeoutId = setTimeout(() => {
      checkForUpdates().catch(e => {
        console.warn("Silent update check failed:", e);
      });
    }, 3000);

    // Periodic check every 5 hours since the app runs continuously
    const intervalId = setInterval(() => {
      checkForUpdates().catch(e => {
        console.warn("Periodic update check failed:", e);
      });
    }, 5 * 60 * 60 * 1000);

    return () => {
      clearTimeout(timeoutId);
      clearInterval(intervalId);
    };
  }, []);

  const checkForUpdates = async (): Promise<UpdateCheckResult | null> => {
    if (isChecking) return null;
    setIsChecking(true);
    
    console.log("======================================");
    console.log("[UPDATER] Application version:", currentVersion);
    console.log("[UPDATER] Updater initialized: true");
    console.log("[UPDATER] Endpoint: https://github.com/V2P-Dev/Vispeak/releases/latest/download/latest.json");
    console.log("[UPDATER] Request started");

    try {
      const update = await check();
      
      console.log("[UPDATER] HTTP status: 200 (implied, if check() succeeded)");
      console.log("[UPDATER] Manifest parsed: true");
      console.log("[UPDATER] Latest version:", update?.version || "none");
      console.log("[UPDATER] Current version:", currentVersion);
      console.log("[UPDATER] Update available:", !!update);
      
      if (update) {
        console.log("[UPDATER] Signature validation: pending (done during download)");
        console.log("[UPDATER] Download URL: (hidden in Tauri Update object)");
        
        setUpdate(update);
        setIsChecking(false);
        return { status: "available" };
      } else {
        setIsChecking(false);
        return { status: "up-to-date" };
      }
    } catch (e) {
      setIsChecking(false);
      const errStr = String(e).toLowerCase();
      let status: UpdateStatus = "error";
      let reason = String(e);
      let httpCode = "Unknown";

      if (errStr.includes("404") || errStr.includes("release json")) {
        httpCode = "404";
        if (errStr.includes("latest.json")) {
          status = "up-to-date";
          reason = "latest.json not found";
        } else {
          status = "up-to-date";
          reason = "Releases not published yet";
        }
      } else if (errStr.includes("failed to fetch") || errStr.includes("network") || errStr.includes("timeout") || errStr.includes("dns")) {
        status = "no-internet";
        reason = "Failed to connect to the update server";
      } else if (errStr.includes("500") || errStr.includes("502") || errStr.includes("503")) {
        httpCode = "500+";
        status = "server-unavailable";
        reason = "GitHub server error";
      } else if (errStr.includes("signature")) {
        status = "invalid-signature";
        reason = "Missing or invalid signature";
      }

      console.log("[UPDATER] HTTP:", httpCode);
      console.log("[UPDATER] Reason:", reason);
      console.log("[UPDATER] Real error string:", String(e));
      
      return { status, reason };
    }
  };

  const clearUpdate = () => {
    setUpdate(null);
  };

  return (
    <UpdaterContext.Provider value={{ update, currentVersion, isChecking, checkForUpdates, clearUpdate }}>
      {children}
    </UpdaterContext.Provider>
  );
}

export function useUpdater() {
  const context = useContext(UpdaterContext);
  if (!context) {
    throw new Error("useUpdater must be used within an UpdaterProvider");
  }
  return context;
}
