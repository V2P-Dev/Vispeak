export function applyThemeToDocument(themeChoice?: string | null, accentChoice?: string | null) {
  let resolvedTheme = themeChoice || document.documentElement.getAttribute("data-theme") || "system";
  if (resolvedTheme === "system") {
    resolvedTheme = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  let resolvedAccent = accentChoice || document.documentElement.getAttribute("data-accent") || "orange";

  document.documentElement.setAttribute("data-theme", resolvedTheme);
  document.body.setAttribute("data-theme", resolvedTheme);
  const rootEl = document.getElementById("root");
  if (rootEl) {
    rootEl.setAttribute("data-theme", resolvedTheme);
  }

  document.documentElement.setAttribute("data-accent", resolvedAccent);
  document.body.setAttribute("data-accent", resolvedAccent);
  if (rootEl) {
    rootEl.setAttribute("data-accent", resolvedAccent);
  }
}

