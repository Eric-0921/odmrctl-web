import React, { createContext, useContext, useState, useCallback, useEffect } from "react";

export type GuiMode = "mock";

interface MockModeState {
  mode: GuiMode;
  bannerCollapsed: boolean;
}

interface MockModeContextValue extends MockModeState {
  toggleBanner: () => void;
}

const STORAGE_KEY = "odmr-gui-m0-mode";

const DEFAULT_STATE: MockModeState = {
  mode: "mock",
  bannerCollapsed: false,
};

function loadState(): MockModeState {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (parsed.mode === "mock") {
        return {
          mode: "mock",
          bannerCollapsed: !!parsed.bannerCollapsed,
        };
      }
    }
  } catch {
    // ignore parse errors
  }
  return DEFAULT_STATE;
}

function saveState(state: MockModeState) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // ignore storage errors (e.g. private mode)
  }
}

const MockModeContext = createContext<MockModeContextValue | null>(null);

export function MockModeProvider({ children }: { children: React.ReactNode }) {
  const [state, setState] = useState<MockModeState>(loadState);

  useEffect(() => {
    saveState(state);
  }, [state]);

  const toggleBanner = useCallback(() => {
    setState((prev) => ({ ...prev, bannerCollapsed: !prev.bannerCollapsed }));
  }, []);

  return (
    <MockModeContext.Provider value={{ ...state, toggleBanner }}>
      {children}
    </MockModeContext.Provider>
  );
}

export function useMockMode(): MockModeContextValue {
  const ctx = useContext(MockModeContext);
  if (!ctx) {
    throw new Error("useMockMode must be used inside MockModeProvider");
  }
  return ctx;
}
