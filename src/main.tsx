import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { CrawlEventsProvider } from "./hooks/useCrawlEvents";
import { ToastProvider } from "./hooks/useToasts";
import { ThemeProvider } from "./hooks/useTheme";
import "./styles/index.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ThemeProvider>
      <ToastProvider>
        <CrawlEventsProvider>
          <App />
        </CrawlEventsProvider>
      </ToastProvider>
    </ThemeProvider>
  </React.StrictMode>
);
