import { spawn } from "node:child_process";
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
const child = spawn("tauri", ["build", ...process.argv.slice(2)], {
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
