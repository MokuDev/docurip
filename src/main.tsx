import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { CrawlEventsProvider } from "./hooks/useCrawlEvents";
import { ToastProvider } from "./hooks/useToasts";
import { ThemeProvider } from "./hooks/useTheme";
import { EscapeStackProvider } from "./contexts/EscapeStack";
import "./styles/index.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ToastProvider>
      <ThemeProvider>
        <EscapeStackProvider>
          <CrawlEventsProvider>
            <App />
          </CrawlEventsProvider>
        </EscapeStackProvider>
      </ThemeProvider>
    </ToastProvider>
  </React.StrictMode>
);
