export function applyThemeToDocument(themeChoice?: string | null) {
  let resolvedTheme = themeChoice || "system";
  if (resolvedTheme === "system") {
    resolvedTheme = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  document.documentElement.setAttribute("data-theme", resolvedTheme);
  document.body.setAttribute("data-theme", resolvedTheme);
  const rootEl = document.getElementById("root");
  if (rootEl) {
    rootEl.setAttribute("data-theme", resolvedTheme);
  }
}
