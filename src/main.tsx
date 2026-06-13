import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { CrawlEventsProvider } from "./hooks/useCrawlEvents";
import "./styles/index.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <CrawlEventsProvider>
      <App />
    </CrawlEventsProvider>
  </React.StrictMode>
);
