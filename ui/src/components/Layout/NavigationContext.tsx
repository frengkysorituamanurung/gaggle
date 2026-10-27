import React, { createContext, ReactNode, useCallback, useContext, useEffect, useRef, useState } from "react";

import { NAV_DIMENSIONS } from "./constants";

const MIN_NAV_WIDTH = 160;
const MAX_NAV_WIDTH = 400;
const NARROW_WINDOW_THRESHOLD = 700;

interface NavigationContextValue {
  isNavExpanded: boolean;
  setIsNavExpanded: (expanded: boolean) => void;
  navWidth: number;
  setNavWidth: (width: number) => void;
}

const NavigationContext = createContext<NavigationContextValue | null>(null);

export const useNavigationContext = () => {
  const context = useContext(NavigationContext);
  if (!context) throw new Error("useNavigationContext must be used within NavigationProvider");
  return context;
};

export const useNavigationContextSafe = () => useContext(NavigationContext);

interface NavigationProviderProps {
  children: ReactNode;
}

export const NavigationProvider: React.FC<NavigationProviderProps> = ({ children }) => {
  const [isNavExpanded, setIsNavExpandedState] = useState<boolean>(() => {
    const stored = localStorage.getItem("navigation_expanded");
    return stored !== "false";
  });

  const setIsNavExpanded = useCallback((expanded: boolean) => {
    setIsNavExpandedState(expanded);
    localStorage.setItem("navigation_expanded", String(expanded));
  }, []);

  const [navWidth, setNavWidthState] = useState<number>(() => {
    const stored = localStorage.getItem("navigation_width");
    if (stored) {
      const parsed = parseInt(stored, 10);
      if (!isNaN(parsed) && parsed >= MIN_NAV_WIDTH && parsed <= MAX_NAV_WIDTH) return parsed;
    }
    return NAV_DIMENSIONS.NAV_WIDTH;
  });

  const setNavWidth = useCallback((width: number) => {
    const clamped = Math.min(MAX_NAV_WIDTH, Math.max(MIN_NAV_WIDTH, width));
    setNavWidthState(clamped);
    localStorage.setItem("navigation_width", String(clamped));
  }, []);

  const isNavExpandedRef = useRef(isNavExpanded);
  useEffect(() => {
    isNavExpandedRef.current = isNavExpanded;
  }, [isNavExpanded]);

  useEffect(() => {
    let lastWidth = window.innerWidth;
    if (lastWidth < NARROW_WINDOW_THRESHOLD && isNavExpandedRef.current) {
      setIsNavExpanded(false);
    }
    const onResize = () => {
      const width = window.innerWidth;
      if (width < NARROW_WINDOW_THRESHOLD && lastWidth >= NARROW_WINDOW_THRESHOLD && isNavExpandedRef.current) {
        setIsNavExpanded(false);
      }
      lastWidth = width;
    };
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, [setIsNavExpanded]);

  const value: NavigationContextValue = { isNavExpanded, setIsNavExpanded, navWidth, setNavWidth };
  return <NavigationContext.Provider value={value}>{children}</NavigationContext.Provider>;
};