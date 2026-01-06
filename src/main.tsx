import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

try {
  const rootElement = document.getElementById("root");
  if (!rootElement) {
    throw new Error("Root element not found");
  }

  ReactDOM.createRoot(rootElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
} catch (error) {
  const rootElement = document.getElementById("root");
  if (rootElement) {
    rootElement.innerHTML = `
      <div style="padding: 20px; font-family: sans-serif;">
        <h1>应用加载失败</h1>
        <p>错误信息: ${error instanceof Error ? error.message : String(error)}</p>
        <p>请查看控制台获取更多信息。</p>
      </div>
    `;
  }
}
