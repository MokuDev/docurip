import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { CrawlEventsProvider } from "./hooks/useCrawlEvents";
import { ToastProvider } from "./hooks/useToasts";
import "./styles/index.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ToastProvider>
      <CrawlEventsProvider>
        <App />
      </CrawlEventsProvider>
    </ToastProvider>
  </React.StrictMode>
);
