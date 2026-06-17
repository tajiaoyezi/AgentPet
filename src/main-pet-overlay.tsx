import React from "react";
import ReactDOM from "react-dom/client";
import PetOverlay from "./windows/PetOverlay";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <PetOverlay />
  </React.StrictMode>,
);
