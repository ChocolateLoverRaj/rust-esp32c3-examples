import React from "react";
import { createRoot } from "react-dom/client";
import { WasmReact, App } from "../pkg/index"

if ('serviceWorker' in navigator) {
  navigator.serviceWorker.register('/service-worker.js').then(registration => {

    console.log('SW registered: ', registration);

  }).catch(registrationError => {

    console.log('SW registration failed: ', registrationError);

  });
}

WasmReact.useReact(React); // Tell wasm-react to use your React runtime

const root = createRoot(document.getElementById("root"));
root.render(<App />);
