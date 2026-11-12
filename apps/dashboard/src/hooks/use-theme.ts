import { useCallback, useEffect, useState } from "react";

export type Theme = "light" | "dark";

const STORAGE_KEY = "kira-theme";
const DARK_QUERY = "(prefers-color-scheme: dark)";

function storedTheme(): Theme | null {
  const value = localStorage.getItem(STORAGE_KEY);
  return value === "light" || value === "dark" ? value : null;
}

function applyTheme(theme: Theme) {
  document.documentElement.classList.toggle("dark", theme === "dark");
}

function applyThemeWithoutTransition(theme: Theme) {
  const root = document.documentElement;

  root.classList.add("disable-transitions");
  applyTheme(theme);

  // Force style recalculation before restoring transitions.
  void root.offsetHeight;

  root.classList.remove("disable-transitions");
}

export function useTheme() {
  const [theme, setTheme] = useState<Theme>(() => {
    const stored = storedTheme();
    if (stored) {
      return stored;
    }

    return document.documentElement.classList.contains("dark") ? "dark" : "light";
  });

  useEffect(() => {
    if (storedTheme()) {
      return;
    }

    const media = window.matchMedia(DARK_QUERY);
    const onChange = (event: MediaQueryListEvent) => {
      if (storedTheme()) {
        return;
      }

      const next: Theme = event.matches ? "dark" : "light";
      applyTheme(next);
      setTheme(next);
    };

    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, []);

  const toggle = useCallback(() => {
    setTheme((current) => {
      const next: Theme = current === "dark" ? "light" : "dark";
      applyThemeWithoutTransition(next);
      localStorage.setItem(STORAGE_KEY, next);
      return next;
    });
  }, []);

  return { theme, toggle };
}
