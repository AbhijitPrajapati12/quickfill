import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import Manager from "./manager/Manager";
import Picker from "./picker/Picker";
import "./styles.css";

// Both windows load this bundle; the window label decides which UI to show.
const isPicker = getCurrentWindow().label === "picker";
document.documentElement.dataset.window = isPicker ? "picker" : "main";

createRoot(document.getElementById("root")!).render(
  <StrictMode>{isPicker ? <Picker /> : <Manager />}</StrictMode>,
);
