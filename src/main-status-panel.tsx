import React from "react";
import ReactDOM from "react-dom/client";
import StatusPanel from "./windows/StatusPanel";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <StatusPanel />
  </React.StrictMode>,
);
