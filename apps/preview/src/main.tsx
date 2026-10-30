import React from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./app.css";

const mount = document.getElementById("root");
if (mount === null) {
  throw new Error("Zoomouse preview root not found");
}

createRoot(mount).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
