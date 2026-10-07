import { spawn, execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { resolve } from "node:path";

if (process.platform !== "linux") {
  console.error("Linux packages must be built on Linux.");
  process.exit(1);
}

// Static GGML builds otherwise use -march=native, tying packages to the builder's CPU.
const cpuFlags = {
  GGML_NATIVE: "OFF",
  GGML_AVX: "OFF",
  GGML_AVX2: "OFF",
  GGML_F16C: "OFF",
  GGML_FMA: "OFF",
};
const cmakeFlags = Object.entries(cpuFlags).map(([key, value]) => `-D${key}=${value}`).join(" ");
const buildArgs = process.argv.slice(2);
if (process.env.VISPEAK_BUILD_KWIN_CARET === "1") {
  // Build against this distribution's KWin headers, never reuse a dev module.
  const bridgeBuild = resolve("src-tauri/target/linux/kwin-caret");
  execFileSync("cmake", ["-S", "packaging/linux/kwin-caret", "-B", bridgeBuild], { stdio: "inherit" });
  execFileSync("cmake", ["--build", bridgeBuild], { stdio: "inherit" });
  execFileSync("ctest", ["--test-dir", bridgeBuild, "--output-on-failure"], { stdio: "inherit" });
  const module = resolve(bridgeBuild, "Vispeak/CaretBridge");
  writeFileSync(resolve(module, "kwin-version"), execFileSync(resolve(bridgeBuild, "caret-bridge-version")));
  buildArgs.unshift("--config", JSON.stringify({ bundle: { resources: {
    [`${module}/`]: "kwin-caret/Vispeak/CaretBridge/",
  } } }));
}
const child = spawn("tauri", ["build", ...buildArgs], {
  stdio: "inherit",
  env: {
    ...process.env,
    ...cpuFlags,
    // whisper-rs-sys does not track GGML environment changes in Cargo. Keep
    // package builds separate from caches produced by ordinary tauri builds.
    CARGO_TARGET_DIR: resolve(process.env.CARGO_TARGET_DIR ?? "src-tauri/target", "linux"),
    TRANSCRIBE_CMAKE_ARGS: `${process.env.TRANSCRIBE_CMAKE_ARGS ?? ""} ${cmakeFlags}`.trim(),
  },
});
child.on("error", error => { console.error(error.message); process.exitCode = 1; });
child.on("exit", code => { process.exitCode = code ?? 1; });
